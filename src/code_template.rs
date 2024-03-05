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
struct Task {
    id: u64,
    name: String,
    completed: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct User {
    id: u64,
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Database {
    tasks: HashMap<u64, Task>,
    users: HashMap<u64, User>,
    
}


impl Database {
    fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            users: HashMap::new(),
        }
    }

    // CRUD DATA
    fn insert(&mut self, task: Task) {
        self.tasks.insert(task.id, task);
    }
    
    fn get(&self, id: &u64) -> Option<&Task> {
        self.tasks.get(id)
    }

    fn get_all(&self) -> Vec<&Task> {
        self.tasks.values().collect() 
    }

    fn delete(&mut self, id: &u64) {
        self.tasks.remove(id);
    }

    fn update(&mut self, task: Task) {
        self.tasks.insert(task.id, task);
    }

    // USER DATA RELATED FUNCTIONS
    fn insert_user(&mut self, user: User) {
        self.users.insert(user.id, user);
    }

    fn get_user_by_name(&self, username: &String) -> Option<&User> {
        self.users.values().find(|user| user.username == *username)
    }
    fn _get_user(&self, id: &u64) -> Option<&User> {
        self.users.get(id)
    }

    fn _get_all_users(&self) -> Vec<&User> {
        self.users.values().collect() 
    }

    // DATABASE SAVING
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

async fn create_task(app_state: web::Data<AppState>, task: web::Json<Task>) -> impl Responder {
    let mut db = app_state.db.lock().unwrap();
    db.insert(task.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().finish()
}

async fn update_task(app_state: web::Data<AppState>, task: web::Json<Task>) -> impl Responder {
    let mut db = app_state.db.lock().unwrap();
    db.update(task.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().finish()
}

async fn read_task(app_state: web::Data<AppState>, id: web::Path<u64>) -> impl Responder {
    let db = app_state.db.lock().unwrap();
    match db.get(&id.into_inner()) {
        Some(task) => HttpResponse::Ok().json(task),
        None => HttpResponse::NotFound().finish(),    
    }
 
}

async fn read_all_task(app_state: web::Data<AppState>) -> impl Responder {
    let db = app_state.db.lock().unwrap();
    let tasks = db.get_all();
    HttpResponse::Ok().json(tasks)
}

async fn delete_task(app_state: web::Data<AppState>, id: web::Path<u64>) -> impl Responder {
    let mut db = app_state.db.lock().unwrap();
    db.delete(&id.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().finish() 
}

async fn register_user(app_state: web::Data<AppState>, user: web::Json<User>) -> impl Responder {
    let mut db = app_state.db.lock().unwrap();
    db.insert_user(user.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().finish()
}

async fn login_user(app_state: web::Data<AppState>, user: web::Json<User>) -> impl Responder {
    let db = app_state.db.lock().unwrap();
    match db.get_user_by_name(&user.username) {
        Some(u) => {
            if u.password == user.password {
                HttpResponse::Ok().body("Login successful")
            } else {
                HttpResponse::BadRequest().body("Invalid password")
            }
        },
        None => HttpResponse::BadRequest().body("Invalid username")
    }

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
            .app_data(data.clone()) //kind of pointer, not deep copy
            .route("/task", web::post().to(create_task))
            .route("/task", web::get().to(read_all_task))
            .route("/task", web::put().to(update_task))
            .route("/task/{id}", web::get().to(read_task))            
            .route("/task/{id}", web::delete().to(delete_task))
            .route("/register", web::post().to(register_user))
            .route("/login", web::post().to(login_user))
            
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await

}