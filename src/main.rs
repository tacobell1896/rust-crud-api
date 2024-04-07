use postgres::Error as PostgresError;
use postgres::{Client, NoTls};
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[macro_use]
extern crate serde_derive;

//Model: Videogame struct
#[derive(Serialize, Deserialize)]
struct Videogame {
    id: Option<i32>,
    name: String,
    description: String,
    rating: i32,
    content_rating: String,
    developer: String,
    publisher: String,
    platform: String,
    genre: String,
    release_date: String,
}

//Database connection
const DB_URL: &str = env!("DATABASE_URL");

//constants
const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n";
const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const INTERNAL_ERROR: &str = "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";

//main function
fn main() {
    //Set Database
    if let Err(_) = set_database() {
        panic!("Error setting up database");
    }

    //start server and print port
    let listener = TcpListener::bind(format!("0.0.0.0:8080")).unwrap();
    println!("Server started at port 8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

//handle_client function
fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer) {
        Ok(size) => {
            request.push_str(&String::from_utf8_lossy(&buffer[..size]).as_ref());

            let (status_line, content) = match &*request {
                r if r.starts_with("POST /videogames") => handle_post_request(&r),
                r if r.starts_with("GET /videogames") => handle_get_all_request(&r),
                r if r.starts_with("GET /videogames/") => handle_get_request(&r),
                r if r.starts_with("PUT /videogames") => handle_put_request(&r),
                r if r.starts_with("DELETE /videogames") => handle_delete_request(&r),
                _ => (NOT_FOUND.to_string(), "Not Found".to_string()),
            };

            stream
                .write_all(format!("{}{}", status_line, content).as_bytes())
                .unwrap();
        }
        Err(e) => {
            eprintln!("Error, unable to read stream: {}", e);
        }
    }
}

//handle_post_request function
fn handle_post_request(request: &str) -> (String, String) {
    match (
        get_videogame_request_body(&request),
        Client::connect(DB_URL, NoTls),
    ) {
        (Ok(videogame), Ok(mut client)) => {
            client
                .execute(
                    "INSERT INTO videogames (name, description, rating, content_rating, developer, publisher, platform, genre, release_date) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                    &[&videogame.name, &videogame.description, &videogame.rating, &videogame.content_rating, &videogame.developer, &videogame.publisher, &videogame.platform, &videogame.genre, &videogame.release_date]
                )
                .unwrap();
            (OK_RESPONSE.to_string(), "Videogame added".to_string())
        }
        _ => (
            INTERNAL_ERROR.to_string(),
            "Error adding videogame".to_string(),
        ),
    }
}

//handle get request function
fn handle_get_request(request: &str) -> (String, String) {
    match (
        get_id(&request).parse::<i32>(),
        Client::connect(DB_URL, NoTls),
    ) {
        (Ok(id), Ok(mut client)) => {
            match client.query("SELECT * FROM videogames WHERE id = $1", &[&id]) {
                Ok(row) => {
                    let videogame = Videogame {
                        id: row[0].get(0),
                        name: row[0].get(1),
                        description: row[0].get(2),
                        rating: row[0].get(3),
                        content_rating: row[0].get(4),
                        developer: row[0].get(5),
                        publisher: row[0].get(6),
                        platform: row[0].get(7),
                        genre: row[0].get(8),
                        release_date: row[0].get(9),
                    };
                    (
                        OK_RESPONSE.to_string(),
                        serde_json::to_string(&videogame).unwrap(),
                    )
                }
                _ => (NOT_FOUND.to_string(), "Videogame not found".to_string()),
            }
        }
        _ => (INTERNAL_ERROR.to_string(), "Error parsing id".to_string()),
    }
}

//handle delete request function
fn handle_delete_request(request: &str) -> (String, String) {
    match (
        get_id(&request).parse::<i32>(),
        Client::connect(DB_URL, NoTls),
    ) {
        (Ok(id), Ok(mut client)) => {
            let rows_affected = client
                .execute("DELETE FROM videogames WHERE id = $1", &[&id])
                .unwrap();
            if rows_affected == 0 {
                return (NOT_FOUND.to_string(), "Video Game not found".to_string());
            }
            (OK_RESPONSE.to_string(), "Videogame deleted".to_string())
        }
        _ => (
            INTERNAL_ERROR.to_string(),
            "Error deleting videogame".to_string(),
        ),
    }
}

//handle_get_all_request function
fn handle_get_all_request(_request: &str) -> (String, String) {
    match Client::connect(DB_URL, NoTls) {
        Ok(mut client) => {
            let mut videogames = Vec::new();

            for row in client.query("SELECT id, name, description, rating, content_rating, developer, publisher, platform, genre, release_date FROM videogames", &[]).unwrap() {
              videogames.push(Videogame {
                            id: row.get(0),
                            name: row.get(1),
                            description: row.get(2),
                            rating: row.get(3),
                            content_rating: row.get(4),
                            developer: row.get(5),
                            publisher: row.get(6),
                            platform: row.get(7),
                            genre: row.get(8),
                            release_date: row.get(9),
                        });
          }
            (
                OK_RESPONSE.to_string(),
                serde_json::to_string(&videogames).unwrap(),
            )
        }
        _ => (
            INTERNAL_ERROR.to_string(),
            "Error getting videogames".to_string(),
        ),
    }
}

//handle_put_request function
fn handle_put_request(request: &str) -> (String, String) {
    match (
        get_id(&request).parse::<i32>(),
        get_videogame_request_body(&request),
        Client::connect(DB_URL, NoTls),
    ) {
        (Ok(id), Ok(videogame), Ok(mut client)) => {
            client
               .execute(
                   "UPDATE videogames SET name = $1, description = $2, rating = $3, content_rating = $4, developer = $5, publisher = $6, platform = $7, genre = $8, release_date = $9 WHERE id = $10",
                   &[&videogame.name, &videogame.description, &videogame.rating, &videogame.content_rating, &videogame.developer, &videogame.publisher, &videogame.platform, &videogame.genre, &videogame.release_date, &id]
               )
               .unwrap();
            (OK_RESPONSE.to_string(), "Videogame updated".to_string())
        }
        _ => (
            INTERNAL_ERROR.to_string(),
            "Error updating videogame".to_string(),
        ),
    }
}

//set_database function
fn set_database() -> Result<(), PostgresError> {
    let url = DB_URL;
    let mut client = Client::connect(url, NoTls)?;
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS videogames (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            description TEXT NULL,
            rating INTEGER NULL,
            content_rating VARCHAR NULL,
            developer VARCHAR NULL,
            publisher VARCHAR NULL,
            platform VARCHAR NULL,
            genre VARCHAR NULL,
            release_date VARCHAR NULL
        )",
    )?;
    Ok(())
}

//get id from request url
fn get_id(request: &str) -> &str {
    request
        .split("/")
        .nth(2)
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or_default()
}

//deserialize videogame from request body without id
fn get_videogame_request_body(request: &str) -> Result<Videogame, serde_json::Error> {
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}
