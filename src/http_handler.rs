use std::collections::HashMap;

use lambda_http::{Body, Error, Request, Response};
use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::{types::AttributeValue, Client as DynamoClient, Error as DynamoError};
use aws_sdk_ses::{
    operation::send_email::SendEmailOutput,
    types::{Body as EmailBody, Content, Destination, Message},
    Client as SesClient, Error as SesError
};
use uuid::Uuid;

pub(crate) async fn function_handler(_event: Request) -> Result<Response<Body>, Error> {

    let config = aws_config::defaults(BehaviorVersion::latest())
        .region("eu-west-2")
        .load()
        .await;

    let dynamo_client = DynamoClient::new(&config);
    let ses_client = SesClient::new(&config);

    add_email(&dynamo_client, String::from("dev@aidanlowson.com")).await?;

    let raw_emails = get_all_items(&dynamo_client, &String::from("Emails")).await?;

    let emails: Vec<String> = raw_emails
        .iter()
        .filter_map(|item| {
            item.get("Subscribed")
                .and_then(|subscribed_object| subscribed_object.as_bool().ok())
                .map(|subbed| *subbed)
                .filter(|subbed| *subbed)
                .and_then(|_| {
                    item.get("Email")
                        .and_then(|email_object| match email_object.as_s() {
                            Ok(s) => Some(s.to_string()),
                            Err(_) => None,
                        })
                })
        })
    .collect();

    println!("Emails: {:?} ", emails);

    send_email(&ses_client, emails).await?;

    println!("Emails sent!");

   let resp = Response::builder()
        .status(200)
        .header("content-type", "text/html")
        .body(String::from("Well done, you sent out some emails from AWS SES using the AWS SDK, powered by Rust!").into())
        .map_err(Box::new)?;
    Ok(resp)
}

async fn add_email(
    client: &DynamoClient,
    email: String,
) -> Result<aws_sdk_dynamodb::operation::put_item::PutItemOutput, DynamoError> {
    let id_av = AttributeValue::S(Uuid::new_v4().to_string());
    let email_av = AttributeValue::S(email);
    let subscribed_av = AttributeValue::Bool(true);

    let req = client
        .put_item()
        .table_name("Emails")
        .item("Id", id_av)
        .item("Email", email_av)
        .item("Subscribed", subscribed_av);

    println!("Executing request [{req:?}] to add an item...");

    let res = req.send().await?;

    println!("Added email!");

    Ok(res)
}

async fn get_all_items(
    client: &DynamoClient,
    table_name: &String
) -> Result<Vec<HashMap<String, AttributeValue>>, DynamoError>  {
    let mut items = Vec::new();
    let mut last_evaluated_key = None;

    loop {
        let resp = client
            .scan()
            .table_name(table_name)
            .set_exclusive_start_key(last_evaluated_key)
            .send()
            .await?;

        if let Some(new_items) = resp.items {
            items.extend(new_items);
        }

        last_evaluated_key = resp.last_evaluated_key;

        if last_evaluated_key.is_none() {
            break;
        }
    }

    Ok(items)
}

async fn send_email(ses_client: &SesClient, recipients: Vec<String>) -> Result<SendEmailOutput, SesError> {
    let sender = "dev@aidanlowson.com"; // Put in any email address you have configured in AWS SES
    let subject = String::from("Hello From AWS Lambda!");
    let body_text = String::from("Writen in Rust, served on AWS, pretty cool ey?");
    let body_html = String::from("<html><body><h1>Writen in Rust, served on AWS, pretty cool ey?</h1></body></html>");

    let destination = Destination::builder()
        .set_bcc_addresses(Some(recipients))
        .build();

    let send_email_builder = ses_client
        .send_email()
        .destination(destination)
        .message(
            Message::builder()
                .subject(Content::builder().data(subject).build()?)
                .body(
                    EmailBody::builder()
                        .text(Content::builder().data(body_text).build()?)
                        .html(Content::builder().data(body_html).build()?)
                        .build(),
                )
                .build(),
        )
        .source(sender);


    let response = send_email_builder.send().await?;

    Ok(response)
}
