pub struct LogCollector {
    logs: Vec<String>,
}
impl LogCollector {
    pub fn new() -> LogCollector {
        LogCollector { logs: Vec::new() }
    }

    pub fn ingest(&mut self, log: String) {
        self.logs.push(log);
    }

    pub fn get(&self) -> &Vec<String> {
        &self.logs
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_collector() {
        let mut collector = LogCollector::new();
        collector.ingest("test log 1".to_string());
        collector.ingest("test log 2".to_string());

        let logs = collector.get();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0], "test log 1");
        assert_eq!(logs[1], "test log 2");
    }

    #[test]
    fn test_empty_collector() {
        let collector = LogCollector::new();
        assert_eq!(collector.get().len(), 0);
    }
}
