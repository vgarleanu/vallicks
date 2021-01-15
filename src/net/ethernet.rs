type TxQueueSender = UnboundedSender<Ether2Frame>;

pub struct Ethernet {
    tx_queue_map: RwLock<HashMap<Mac, TxQueueSender>>,
}

impl Ethernet {
    pub const fn new() -> Self {
        Self {
            tx_queue_map: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_tx(&self, device_mac: Mac, tx_queue: TxQueueSender) {
        self.tx_queue_map
            .write()
            .await
            .and_then(|mut x| x.insert(device_mac, tx_queue));
    }

    /// Function handles an incoming packet.
    pub fn handle_rx(&self, packet: Ether2Frame) -> Option<Ether2Frame> {
        todo!()
    }

    /// Function can be used to send data out.
    pub fn handle_tx(&self, packet: Ether2Frame) {
        self.tx_queue_map
            .read()
            .await
            .and_then(|x| x.get(&packet.dst).and_then(|x| x.send(packet)));
    }
}
