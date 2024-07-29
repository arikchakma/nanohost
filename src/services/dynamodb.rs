use std::collections::HashMap;

use aws_config::SdkConfig as AwsConfig;
use aws_sdk_dynamodb::{types::AttributeValue, Client as DynamodbClient};

#[derive(Debug, Clone)]
pub struct Client {
    dynamodb: DynamodbClient,
    table_name: String,
}

impl Client {
    pub fn new(config: &AwsConfig, table_name: &str) -> Client {
        Client {
            dynamodb: DynamodbClient::new(config),
            table_name: table_name.to_string(),
        }
    }

    pub async fn put_item(&self, item: HashMap<String, AttributeValue>) -> Result<(), ()> {
        let mut input = self.dynamodb.put_item().table_name(self.table_name.clone());
        for (key, value) in item {
            input = input.item(key, value);
        }

        let _response = input.send().await.expect("Failed to put item");

        Ok(())
    }
}
