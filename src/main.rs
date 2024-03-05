use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpResponse, HttpServer, Responder};
use serde::{Serialize,Deserialize};
use serde_json;
use reqwest::Client as HttpClient;
use async_trait::async_trait;
use std::sync::Mutex;
use std::collections::HashMap;
use std::fs;
use std::io::Write;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Crypto {
    id: u64,
    name: String,
    price: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Database {
    cryptos: HashMap<u64, Crypto>,
}

impl Database {
    fn new() -> Self {
        Self {
            cryptos: HashMap::new(),
        }
    }

    fn insert(&mut self, crypto: Crypto) {
        self.cryptos.insert(crypto.id, crypto);
    }
    
    fn get(&self, id: &u64) -> Option<&Crypto> {
        self.cryptos.get(id)
    }

    fn get_all(&self) -> Vec<&Crypto> {
        self.cryptos.values().collect() 
    }

    fn update(&mut self, crypto: Crypto) {
        self.cryptos.insert(crypto.id, crypto);
    }

    fn save_to_file(&self) -> std::io::Result<()> {
        let data = serde_json::to_string(&self)?;
        let mut file = fs::File::create("database.json")?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    fn load_from_file() -> std::io::Result<Self> {
        let file_content = fs::read_to_string("database.json")?;
        let data: Database = serde_json::from_str(&file_content)?;
        Ok(data)
    }

}

struct AppState {
    db: Mutex<Database>
}

async fn create_crypto(app_state: web::Data<AppState>, crypto: web::Json<Crypto>) -> impl Responder {
    let mut db = app_state.db.lock().unwrap();
    db.insert(crypto.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().finish()
}

async fn update_crypto(app_state: web::Data<AppState>, crypto: web::Json<Crypto>) -> impl Responder {
    let mut db = app_state.db.lock().unwrap();
    db.update(crypto.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().finish()
}

async fn read_crypto(app_state: web::Data<AppState>, id: web::Path<u64>) -> impl Responder {
    let db = app_state.db.lock().unwrap();
    match db.get(&id.into_inner()) {
        Some(crypto) => HttpResponse::Ok().json(crypto),
        None => HttpResponse::NotFound().finish(),    
    }
 
}

async fn read_all_crypto(app_state: web::Data<AppState>) -> impl Responder {
    let db = app_state.db.lock().unwrap();
    let cryptos = db.get_all();
    HttpResponse::Ok().json(cryptos)
}

async fn fetch_data_from_external_sources(app_state: web::Data<AppState>) {    
    let client = HttpClient::new();
    let response = client.get("https://api.binance.com/api/v3/ticker/price").send().await.unwrap();
    
    let cryptos: Vec<Crypto> = response.json().await.unwrap();
    let mut db = app_state.db.lock().unwrap();
    for crypto in cryptos {
        db.insert(crypto);
    }
    let _ = db.save_to_file();
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = match Database::load_from_file() {
        Ok(db) => db,
        Err(_) => Database::new(),
    };

    let data: web::Data<AppState> = web::Data::new(AppState {
        db: Mutex::new(db)
    });

    fetch_data_from_external_sources(data.clone()).await;

    HttpServer::new( move || {
        App::new()
            .wrap(
                Cors::permissive()
                    .allowed_origin_fn(|origin, _req_head| {
                        origin.as_bytes().starts_with(b"http://localhost") || origin == "null"
                    })
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .max_age(3600)
                    .supports_credentials()                    
            )
            .app_data(data.clone()) 
            .route("/crypto", web::post().to(create_crypto))
            .route("/crypto", web::get().to(read_all_crypto))
            .route("/crypto", web::put().to(update_crypto))
            .route("/crypto/{id}", web::get().to(read_crypto))            
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}