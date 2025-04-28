use aws_sdk_resourcegroupstagging::types::TagFilter;
use aws_sdk_resourcegroupstagging::{Client, Error};
use tokio;

pub async fn all_beez_tags() -> Result<(), Error> {
    println!("[all_beez_tags] hey");

    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2025_01_17()).await;
    println!("[all_beez_tags] config");

    let client = Client::new(&config);
    println!("[all_beez_tags] client");

    let tag_filter = TagFilter::builder()
        .key("Name")
        .values("killabeez-test")
        .build();
    println!("[all_beez_tags] tag_filter");

    let result = match client.get_resources().tag_filters(tag_filter).send().await {
        Ok(r) => r,
        Err(e) => panic!(
            "[all_beez_tags] ERROR: Can't load tagged resources\n{:?}",
            e
        ),
    };
    println!("[all_beez_tags] loaded tagged resources");

    if let Some(resources) = result.resource_tag_mapping_list {
        for resource in resources {
            if let Some(arn) = resource.resource_arn {
                println!("Found resource: {}", arn);
            }
        }
    } else {
        println!("No resources found with the specified tag.");
    }

    Ok(())
}
