use interface::Interface;
use jsonrpc_http_server::{jsonrpc_core::IoHandler, ServerBuilder};
use std::sync::mpsc::Receiver;

pub struct HttpInterface {
    port: u16,
}

impl HttpInterface {
    pub fn new(port: u16) -> Self {
        HttpInterface { port }
    }
}

impl Interface for HttpInterface {
    fn run(&self, handler: IoHandler, kill_switch: Receiver<()>) -> Result<(), String> {
        let url = format!("0.0.0.0:{}", self.port);
        let _server = ServerBuilder::new(handler)
            .start_http(&url.parse().expect("Invalid URL!"))
            .map_err(|e| e.to_string())?;
        let _ = kill_switch.recv();
        Ok(())
    }
}
