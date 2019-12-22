use serde::{Deserialize, Serialize};
use log::{trace};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GBFTConfig{
    version: String,
    vc_retry_times: u64,
    recovery_retry_times: u64,
}

impl GBFTConfig {
    pub fn get_version(&self) -> String{
        trace!("config version is {:?}", self.version);
        return self.version.clone();
    }

}

#[test]
fn test_get_version(){
    let config = GBFTConfig{
        version: String::from("1.0.1"),
        vc_retry_times: 10,
        recovery_retry_times: 12,
    };

    assert_eq!(config.get_version(), "1.0.1");

}
