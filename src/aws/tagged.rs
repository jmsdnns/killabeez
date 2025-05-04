use aws_sdk_resourcegroupstagging::types::TagFilter;
use aws_sdk_resourcegroupstagging::{Client, Error};
use tokio;

use crate::config::SwarmConfig;

pub async fn mk_client() -> Client {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2025_01_17()).await;
    Client::new(&config)
}

pub fn mk_filter(tag_name: &str) -> TagFilter {
    TagFilter::builder().key("Name").values(tag_name).build()
}

pub async fn list_all_tagged(client: &Client, sc: &SwarmConfig) -> Result<Vec<String>, Error> {
    let tag_filter = mk_filter(&sc.tag_name);
    println!("[list_all_tagged] tag_filter");

    let response = match client.get_resources().tag_filters(tag_filter).send().await {
        Ok(r) => r,
        Err(e) => panic!("[list_all_tagged] ERROR: load failed\n{:?}", e),
    };

    let arns = match response.resource_tag_mapping_list {
        Some(resources) => resources
            .iter()
            .map(|r| r.resource_arn.clone().unwrap())
            .collect::<Vec<String>>(),
        None => Vec::new(),
    };

    Ok(arns)
}
