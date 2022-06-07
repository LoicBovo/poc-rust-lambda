use lambda_runtime::{service_fn, LambdaEvent, Error};
use serde_json::{json, Value};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use aws_sdk_dynamodb::{model::AttributeValue, output::{PutItemOutput, GetItemOutput}};

#[derive(Serialize, Deserialize)]
struct Person {
    first_name: String, 
    last_name: String,
    age: u8
}

struct DataTable {
    client: aws_sdk_dynamodb::Client,
    table_name: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = service_fn(func);
    lambda_runtime::run(func).await?;
    Ok(())
}

async fn func(event: LambdaEvent<Person>) -> Result<Value, Error> {
    let table_name :String = "People".to_string();
        
    let (event, _context) = event.into_parts();

    println!("this is the request received: {:?}", _context);

    let person: Person =  event;

    let shared_config = aws_config::from_env().load().await;
    let client = aws_sdk_dynamodb::Client::new(&shared_config);

    let person_json = serde_json::to_value(&person).expect("bad input to serialize");
    let datatable  = DataTable {
        client,
        table_name,
    };

    let put_result = put_item(&datatable,&person_json);

    let get_result = get_item(&datatable, &person);
    
    // normally we should do it sequencially just wanted to use the join feature to try it out
    // gain a bit of performance too 
    let (put_item, get_item) = futures::join!(put_result,get_result);

    put_item.expect("failed inserting data");

    println!("return of the get {:?}", get_item);

    Ok(json!({ "statusCode": 200 ,"message": format!("Hello, {}!", person.first_name) }))
}

async fn put_item(datatable: &DataTable, value: &Value) -> Result<PutItemOutput,Error> {

    let test = datatable.client
        .put_item()
        .table_name(datatable.table_name.clone())
        .set_item(Some(parse_item(value.clone())))
        .send().await?;

    Ok(test)
}

async fn get_item(datatable: &DataTable, person: &Person) -> Result<GetItemOutput, Error> {

    let mut prim_key: HashMap<String, AttributeValue> = HashMap::new();
    
    prim_key.insert("last_name".to_string(), AttributeValue::S(person.last_name.clone()));
    prim_key.insert("first_name".to_string(), AttributeValue::S(person.first_name.clone()));

    let option_prim_key = Some(prim_key);

    let response = datatable.client
        .get_item()
        .table_name(&datatable.table_name)
        .set_key(option_prim_key)
        .send().await?;

        Ok(response)
}

fn parse_item(value: Value) -> HashMap<String, AttributeValue> {
    match value_to_item(value) {
        AttributeValue::M(map) => map,
        other => panic!("can only insert top level values, got {:?}", other),
    }
}

fn value_to_item(value: Value) -> AttributeValue {
    match value {
        Value::Null => AttributeValue::Null(true),
        Value::Bool(b) => AttributeValue::Bool(b),
        Value::Number(n) => AttributeValue::N(n.to_string()),
        Value::String(s) => AttributeValue::S(s),
        Value::Array(a) => AttributeValue::L(a.into_iter().map(value_to_item).collect()),
        Value::Object(o) => {
            AttributeValue::M(o.into_iter().map(|(k, v)| (k, value_to_item(v))).collect())
        }
    }
}