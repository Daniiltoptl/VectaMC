//! # Vecta
//!
//! The main library for the Vecta Minecraft server.

use std::sync::atomic::Ordering;
use std::{
    error::Error,
    fmt, io,
    net::{Ipv4Addr, SocketAddrV4},
    sync::{Arc, OnceLock},
};

use tokio::{net::TcpListener, runtime::Runtime, select};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vecta_core::{
    GIT_HASH_SHORT, command::CommandRegistry, permission::PermissionGroupManager, server::Server,
};
use vecta_login::{JavaTcpClient, ServerConnectionSession};

/// Server configuration module.
pub mod config;
/// A module for logging utilities.
pub mod logger;

/// Static access to the server
pub static SERVER: OnceLock<Arc<Server>> = OnceLock::new();

/// The main server struct.
pub struct VectaServer {
    /// The TCP listener for incoming connections.
    pub tcp_listener: TcpListener,
    /// The cancellation token for graceful shutdown.
    pub cancel_token: CancellationToken,
    /// The next client ID to be assigned.
    pub client_id: u64,
    /// The shared server state.
    pub server: Arc<Server>,
    /// Session id UUID state
    pub connection_session: Arc<ServerConnectionSession>,
}

/// Startup error for expected operational failures.
#[derive(Debug)]
pub enum VectaServerError {
    /// Core server startup failed.
    Core(String),
    /// TCP listener could not bind.
    Bind {
        /// Server port that failed to bind.
        port: u16,
        /// Underlying IO error.
        source: io::Error,
    },
}

impl fmt::Display for VectaServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => f.write_str(error),
            Self::Bind { port, source } => {
                write!(f, "failed to bind to server port {port}: {source}")
            }
        }
    }
}

impl Error for VectaServerError {}

impl VectaServer {
    /// Creates a new Vecta server.
    ///
    pub async fn new(
        chunk_runtime: Arc<Runtime>,
        cancel_token: CancellationToken,
        vecta_config: config::VectaConfig,
    ) -> Result<Self, VectaServerError> {
        Self::new_with_commands(
            chunk_runtime,
            cancel_token,
            vecta_config,
            CommandRegistry::new(),
        )
        .await
    }

    /// Creates a new Vecta server with additional commands registered atomically at startup.
    pub async fn new_with_commands(
        chunk_runtime: Arc<Runtime>,
        cancel_token: CancellationToken,
        vecta_config: config::VectaConfig,
        command_registry: CommandRegistry,
    ) -> Result<Self, VectaServerError> {
        log::info!("Starting VectaMC Server ({GIT_HASH_SHORT})");

        let permission_group_store = vecta_config.permission_group_store();
        let server_port = vecta_config.server.server_port;
        let worlds_config = vecta_config.worlds;
        let permission_groups =
            PermissionGroupManager::new(vecta_config.groups, permission_group_store).map_err(
                |error| {
                    VectaServerError::Core(format!("failed to validate groups config: {error}"))
                },
            )?;
        let player_idle_timeout = vecta_config.server.player_idle_timeout;
        let runtime_config = vecta_config.server.into_runtime_config();

        let server = Server::new_with_commands(
            chunk_runtime,
            cancel_token.clone(),
            runtime_config,
            worlds_config,
            permission_groups,
            command_registry,
        )
        .await
        .map_err(VectaServerError::Core)?;

        server
            .player_idle_timeout
            .store(player_idle_timeout, Ordering::Relaxed);

        let tcp_listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, server_port))
            .await
            .map_err(|source| VectaServerError::Bind {
                port: server_port,
                source,
            })?;

        Ok(Self {
            tcp_listener,
            cancel_token,
            client_id: 0,
            server: Arc::new(server),
            connection_session: Arc::new(ServerConnectionSession::default()),
        })
    }

    /// Starts the server and begins accepting connections.
    pub async fn start(&mut self, task_tracker: TaskTracker) {
        log::info!("Started VectaMC Server");

        let server = Arc::clone(&self.server);
        let token = self.cancel_token.clone();
        let server_handle = tokio::spawn(async move {
            server.run(token).await;
        });

        loop {
            select! {
                () = self.cancel_token.cancelled() => {
                    break;
                }
                accept_result = self.tcp_listener.accept() => {
                    let Ok((connection, address)) = accept_result else {
                        continue;
                    };
                    if let Err(e) = connection.set_nodelay(true) {
                        log::warn!("Failed to set TCP_NODELAY: {e}");
                    }
                    let (java_client, sender_recv, net_reader) = JavaTcpClient::new(
                        connection,
                        address,
                        self.client_id,
                        self.cancel_token.child_token(),
                        Arc::clone(&self.server),
                        Arc::clone(&self.connection_session),
                        task_tracker.clone(),
                    );
                    self.client_id = self.client_id.wrapping_add(1);
                    log::info!("Accepted connection from Java Edition: {address} (id {})", self.client_id);

                    let java_client = Arc::new(java_client);
                    java_client.start_outgoing_packet_task(sender_recv);
                    java_client.start_incoming_packet_task(net_reader);
                    // Java_client won't drop until the incoming and outcoming task close
                    // So we dont need to care about them here anymore
                }
            }
        }
        let _ = server_handle.await;
    }
}
