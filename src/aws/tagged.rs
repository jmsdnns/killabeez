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

pub async fn list_all_tagged(sc: &SwarmConfig) -> Result<(), Error> {
    let client = mk_client().await;
    println!("[list_all_tagged] client");

    let tag_filter = mk_filter(&sc.tag_name);
    println!("[list_all_tagged] tag_filter");

    let response = match client.get_resources().tag_filters(tag_filter).send().await {
        Ok(r) => r,
        Err(e) => panic!("[list_all_tagged] ERROR: load failed\n{:?}", e),
    };

    if let Some(resources) = response.resource_tag_mapping_list {
        match resources.len() {
            0 => println!("[list_all_tagged] 0 resources found"),
            count => {
                println!("[list_all_tagged] {} resources found", count);
                for resource in resources.clone() {
                    if let Some(arn) = resource.resource_arn {
                        println!("{}", arn);
                    }
                }
            }
        };
    } else {
        println!("[list_all_tagged] ERROR: tag mapping is empty");
    }

    Ok(())
}
