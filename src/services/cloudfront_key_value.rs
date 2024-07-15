use aws_config::SdkConfig as AwsConfig;
use aws_sdk_cloudfrontkeyvaluestore::Client as CloudFrontClient;

#[derive(Debug, Clone)]
pub struct Client {
    cloudfront: CloudFrontClient,
    kvs_arn: String,
}

impl Client {
    pub fn new(config: &AwsConfig, kvs_arn: &str) -> Client {
        Client {
            cloudfront: CloudFrontClient::new(config),
            kvs_arn: kvs_arn.to_string(),
        }
    }

    pub async fn get_value(&self, key: &str) -> Result<String, String> {
        let result = self
            .cloudfront
            .get_key()
            .kvs_arn(&self.kvs_arn)
            .key(key)
            .send()
            .await;

        match result {
            Ok(response) => {
                let value = response.value;
                Ok(value)
            }
            Err(e) => {
                println!("Error getting cloudfront key value: {:?}", e);
                Err(e.to_string())
            }
        }
    }

    pub async fn set_value(&self, key: &str, value: &str) -> Result<(), String> {
        println!("Setting cloudfront key value: {} => {}", key, value);
        let e_tag = self
            .cloudfront
            .describe_key_value_store()
            .kvs_arn(&self.kvs_arn)
            .send()
            .await;

        let e_tag = match e_tag {
            Ok(response) => response.e_tag,
            Err(e) => {
                println!("Error getting cloudfront key value: {:#?}", e);
                return Err(e.to_string());
            }
        };

        let result = self
            .cloudfront
            .put_key()
            .key(key)
            .value(value)
            .kvs_arn(&self.kvs_arn)
            .if_match(e_tag)
            .send()
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("Error setting cloudfront key value: {:#?}", e);
                Err(e.to_string())
            }
        }
    }

    pub async fn delete_value(&self, key: &str) -> Result<(), String> {
        let e_tag = self
            .cloudfront
            .describe_key_value_store()
            .kvs_arn(&self.kvs_arn)
            .send()
            .await;

        let e_tag = match e_tag {
            Ok(response) => response.e_tag,
            Err(e) => {
                println!("Error getting cloudfront key value: {:#?}", e);
                return Err(e.to_string());
            }
        };

        let result = self
            .cloudfront
            .delete_key()
            .kvs_arn(&self.kvs_arn)
            .key(key)
            .if_match(e_tag)
            .send()
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("Error deleting cloudfront key value: {:#?}", e);
                Err(e.to_string())
            }
        }
    }
}
