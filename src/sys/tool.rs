use std::{
    io::{Error, Write},
    sync::Arc,
};

use rustls::{pki_types::ServerName, ClientConfig, RootCertStore};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_rustls::TlsConnector;

use super::{
    app::App,
    dbs::adapter::{NoCertificateVerification, DB},
    init::{Addr, DBConfig},
};

/// Different features
#[derive(Debug)]
pub struct Tool {
    pub(crate) stop: Option<(Arc<Addr>, i64)>,
    pub(crate) root: Arc<String>,
    pub(crate) db: Arc<DB>,
    pub(crate) install_end: bool,
}

impl Tool {
    pub(crate) fn new(db: Arc<DB>, stop: Option<(Arc<Addr>, i64)>, root: Arc<String>) -> Tool {
        Tool { stop, root, db, install_end: false }
    }

    /// Stop server after install
    pub fn install_end(&mut self) {
        if !self.db.in_use() {
            self.install_end = true;
        }
    }

    /// Stop server
    pub(crate) fn stop(&mut self) {
        if let Some((rpc, stop)) = self.stop.take() {
            App::stop(rpc, stop);
        }
    }

    /// Get number of avaible CPU
    pub fn get_cpu(&self) -> usize {
        num_cpus::get()
    }

    /// Get path of current exe file
    pub fn get_root(&self) -> Arc<String> {
        Arc::clone(&self.root)
    }

    /// Check db connection
    pub async fn check_db(&self, config: DBConfig, sql: Option<Vec<String>>) -> Result<String, String> {
        DB::check_db(&config, sql).await
    }

    /// Get database type
    pub fn get_db_type(&self) -> &'static str {
        #[cfg(feature = "pgsql")]
        return "PostgreSQL";
        #[cfg(feature = "mssql")]
        return "MS Sql Server";
        #[cfg(not(any(feature = "pgsql", feature = "mssql")))]
        return "Not defined";
    }

    /// Get install sql srcipt from github
    pub async fn get_install_sql(&self) -> Result<String, std::io::Error> {
        let addr = "raw.githubusercontent.com:443";
        let domain = "raw.githubusercontent.com";

        #[cfg(feature = "mssql")]
        let url_path = "/tryteex/tiny-web/refs/heads/main/sql/lib-install-mssql.sql";
        #[cfg(feature = "pgsql")]
        let url_path = "/tryteex/tiny-web/refs/heads/main/sql/lib-install-pgsql.sql";
        #[cfg(not(any(feature = "pgsql", feature = "mssql")))]
        let url_path = "/tryteex/tiny-web/refs/heads/main/sql/lib-install-nosql.sql";

        let stream = TcpStream::connect(addr).await?;

        let mut config = ClientConfig::builder().with_root_certificates(RootCertStore::empty()).with_no_client_auth();
        config.dangerous().set_certificate_verifier(Arc::new(NoCertificateVerification {}));
        let tls_connector = TlsConnector::from(Arc::new(config));
        let server_name = ServerName::try_from(domain).unwrap();

        let mut tls_stream = tls_connector.connect(server_name, stream).await?;

        let request = format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", url_path, domain);
        tls_stream.write_all(request.as_bytes()).await?;

        let mut response = Vec::new();
        tls_stream.read_to_end(&mut response).await?;

        let response_str = String::from_utf8_lossy(&response);
        let sql = if let Some(body_start) = response_str.find("\r\n\r\n") {
            &response_str[body_start + 4..]
        } else {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, ""));
        };

        Ok(sql.to_owned())
    }

    // Save config file
    pub fn save_config_file(&self, data: &str) -> Result<(), Error> {
        let mut file = std::fs::File::create(format!("{}/tiny.toml", self.root))?;
        file.write_all(data.as_bytes())?;

        Ok(())
    }
}
