

use axum::{
    routing::{get, post},
    http::StatusCode,
    Json, Router, Extension,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use mongodb::{Client, options::ClientOptions, Database};
use std::sync::Arc;
use tokio::sync::RwLock;
use axum::response::IntoResponse;
use futures::stream::StreamExt;

#[tokio::main]
async fn main() {
    // Create a MongoDB client
    let client_options = ClientOptions::parse("mongodb+srv://Nidhish:123321@nodeexpressprojects.8ls67qd.mongodb.net/NextAuthPrisma?retryWrites=true&w=majority").await.unwrap();
    let client = Client::with_options(client_options).unwrap();
    let database = client.database("my_database");
    let db = Arc::new(RwLock::new(database));

    // build our application with a route
    let app = Router::new()
        .route("/", get(root))
        .route("/users", post(create_user))
        .layer(Extension(db)); // Add the database to the application state

    // run our app with hyper, listening globally on port 3000
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000)); // Listen on all interfaces
    println!("Listening on {}", addr);

    // Use hyper to run the Axum app
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// basic handler that responds with a static string
// Handler to fetch all users from the "users" collection
async fn root(Extension(db): Extension<Arc<RwLock<Database>>>) -> impl IntoResponse {
    let collection = db.read().await.collection::<User>("users");
    let mut cursor = collection.find(None, None).await.unwrap();

    let mut users = Vec::new();
    while let Some(result) = cursor.next().await {
        match result {
            Ok(user) => users.push(user),
            Err(e) => {
                eprintln!("Error fetching user: {:?}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(Vec::<User>::new())).into_response();
            }
        }
    }

    (StatusCode::OK, Json(users)).into_response()
}

async fn create_user(
    Json(payload): Json<CreateUser>,
    Extension(db): Extension<Arc<RwLock<Database>>>,
) -> impl IntoResponse {
    let user = User {
        id: payload.id,
        username: payload.username.clone(),
    };

    let collection = db.read().await.collection("users");
    let insert_result = collection.insert_one(user.clone(), None).await;
    match insert_result {
        Ok(_) => (StatusCode::CREATED, Json(user)).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(user)).into_response(),
    }
}

#[derive(Deserialize)]
struct CreateUser {
    id: u64,
    username: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct User {
    id: u64,
    username: String,
}
