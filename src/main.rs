use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpServer, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use reqwest::Client as HttpClient;
use async_trait::async_trait;

use std::hash::Hash;
use std::sync::Mutex;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use rand::Rng;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Game {
    id: u64,
    guess: u64,
    number: u64,
    hint: String,
    result: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Database {
    games: HashMap<u64, Game>,
}

impl Database {
    fn new() -> Self{
        Self {
            games: HashMap::new(),
        }
    }

    fn insert(&mut self, game: Game) {
        self.games.insert(game.id, game);
    }

    fn get(&self, id: &u64) -> Option<&Game> {
        self.games.get(id)
    }

    fn get_all(&self) -> Vec<&Game> {
        self.games.values().collect()
    }

    fn delete(&mut self, id: &u64){
        self.games.remove(id);
    }

    fn update(&mut self, game: Game) {
        self.games.insert(game.id, game);
    }

    fn save_to_file(&self) -> std::io::Result<()> {
        let data = serde_json::to_string(&self)?;
        let mut file = fs::File::create("database.json")?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    fn load_from_file() -> std::io::Result<Self> {
        let file_content = fs::read_to_string("database.json")?;
        let db: Self = serde_json::from_str(&file_content)?;
        Ok(db)
    }
}

struct AppState {
    db: Mutex<Database>
}

async fn create_game(app_state: web::Data<AppState>, game: web::Json<Game>) -> impl Responder {
    let mut db = app_state.db.lock().expect("failed to lock database");
    let mut game = game.into_inner();
    game.number = rand::thread_rng().gen_range(1..101);
    game.hint = String::from("Guess a number between 1 and 100");
    game.result = String::from("Game started");
    db.insert(game.clone());
    let _ = db.save_to_file();
    HttpResponse::Ok().json(game)
}

async fn guess_number(app_state: web::Data<AppState>, id: web::Path<u64>, guess: web::Json<Game>) -> impl Responder {
    let db_lock = app_state.db.lock().unwrap();
    let mut db = db_lock;
    if let Some(mut game) = db.get(&id).cloned() {
        if guess.guess < game.number {
            game.hint = "Too low!".to_string();
        } else if guess.guess > game.number {
            game.hint = "Too high!".to_string();
        } else {
            game.hint = "Correct!".to_string();
            game.result = "You won!".to_string();
        }
        db.update(game.clone());
        db.save_to_file().unwrap();
        HttpResponse::Ok().json(game)  // Return the updated game details as JSON
    } else {
        HttpResponse::NotFound().body("Game not found")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()>{
    let db = match Database::load_from_file() {
        Ok(db) => db,
        Err(_) => Database::new()
    };

    let data = web::Data::new(AppState {
        db: Mutex::new(db)
    });

    HttpServer::new(move || {
        App::new()
            .wrap (
                Cors::permissive()
                    .allowed_origin_fn(|origin, _req_head| {
                        origin.as_bytes().starts_with(b"http://localhost") || origin == "null"
                    })
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .supports_credentials()
                    .max_age(3600)
            )
            .app_data(data.clone())
            .route("/game", web::post().to(create_game))
            .route("/game/{id}", web::put().to(guess_number))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}