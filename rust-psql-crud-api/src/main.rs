use postgres::{Client, NoTls, Error as PostgresError};
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::env;
use rand::Rng;

#[macro_use]
extern crate serde_derive;

#[derive(Serialize, Deserialize, Debug)]
struct User 
{
    id: Option<i32>,
    name: String, 
    email: String,
}

//Database URL
const DB_URL: &str = env!("DATABASE_URL");

//constants
const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const INTERNAL_ERROR: &str = "HTTP/1.1 500 INTERNAL ERROR\r\n\r\n";

fn main()
{
    println!("crud application on psql in rust");

    // set database
    if let Err(e) = set_database()
    {
        println!("Error in connecting with Psql DB. Error:{}", e);
        return;
    }

    //start server
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    println!("The server is listening on port: 8080");
    for stream in listener.incoming(){
        match stream
        {
            Ok(stream) => {handle_client(stream)},
            Err(e) => {println!("Unable to connect. Err:{}", e)},
        }
    }
}

fn set_database() -> Result<(), PostgresError>
{
    let mut client = Client::connect(DB_URL, NoTls)?;
    client.batch_execute(        "
        CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            email VARCHAR NOT NULL
        )
    ")?;
    Ok(())
}

fn handle_client(mut stream: TcpStream)
{
    let mut buffer = [0; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer)
    {
        Ok(s) => {
            request.push_str(String::from_utf8_lossy(&buffer[..s]).as_ref());
            println!("request:{}", request);
            
            let (status, content) = match &request
            {
                r if r.starts_with("POST /users") => {handle_post_request(r)},
                r if r.starts_with("GET /users/") => {handle_get_request(r)},
                _ => {(NOT_FOUND.to_string(), "404 not found".to_string())},
            };
            stream.write_all(format!("{}{}", status, content).as_bytes()).unwrap();
        }
        Err(e) => {println!("Unable to read stream. Error:{e}")}
    }
}

fn handle_post_request(req: &str) -> (String, String)
{
    let mut rng = rand::thread_rng();
    let id: i32 = rng.gen_range(1..=1000);
    let id_ref: Option<i32> = Some(id);

    match (get_user_request_body(req), Client::connect(DB_URL, NoTls))
    {
        (Ok(user), Ok(mut client)) => {
            client.execute("INSERT INTO users (id, name, email) VALUES ($1, $2, $3)", &[&id_ref, &user.name, &user.email]).unwrap();
            (OK_RESPONSE.to_string(), "Internal error".to_string())
        },
        _ => {(INTERNAL_ERROR.to_string(), "Error in connecting with psql DB".to_string())},
    }
}

fn handle_get_request(req: &str) -> (String, String)
{
   match (get_id(req).parse::<i32>(), Client::connect(DB_URL, NoTls))
    {
        (Ok(id), Ok(mut client)) => {
            match client.query_one("SELECT * FROM users WHERE id=$1", &[&id]) {
                Ok(row) => {
                    let user = User {
                        id: row.get(0),
                        name: row.get(1),
                        email: row.get(2),
                    };
                    (OK_RESPONSE.to_string(), serde_json::to_string(&user).unwrap())
                } 
                _ => (NOT_FOUND.to_string(), "User not found".to_string()),
            }
        }
        _ => {(INTERNAL_ERROR.to_string(), "Error in connecting with psql DB".to_string())}
    }
}

//Get id from request URL
fn get_id(request: &str) -> &str {
    request.split("/").nth(2).unwrap_or_default().split_whitespace().next().unwrap_or_default()
}

//deserialize user from request body without id
fn get_user_request_body(request: &str) -> Result<User, serde_json::Error> {
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}
