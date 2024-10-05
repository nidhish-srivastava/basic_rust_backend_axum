use axum::{
    routing::{get, post, put, delete},
    http::StatusCode,
    Json, Router, Extension, extract::Path,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use mongodb::{Client, options::ClientOptions, Database, bson::{doc, oid::ObjectId}};
use std::sync::Arc;
use tokio::sync::RwLock;
use axum::response::IntoResponse;
use futures::stream::StreamExt;
use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() {
    // Create a MongoDB client
    dotenv().ok();
    // Read the MongoDB URI from the environment variable
    let mongo_uri = env::var("MONGO_URI").expect("MONGO_URI must be set");
    let client_options = ClientOptions::parse(&mongo_uri).await.unwrap();
    let client = Client::with_options(client_options).unwrap();
    let database = client.database("my_database");
    let db = Arc::new(RwLock::new(database));

    // Build our application with a route
    let app = Router::new()
        .route("/", get(root))
        .route("/users", post(create_user))
        .route("/users/:id", put(update_user))
        .route("/users/:id", delete(delete_user))
        .layer(Extension(db)); // Add the database to the application state

    // Run our app with hyper, listening globally on port 3000
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000)); // Listen on all interfaces
    println!("Listening on {}", addr);

    // Use hyper to run the Axum app
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// Basic handler that responds with a static string
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
        id: ObjectId::new(), // Generate a new ObjectId for the user
        username: payload.username.clone(),
    };

    let collection = db.read().await.collection("users");
    let insert_result = collection.insert_one(user.clone(), None).await;
    match insert_result {
        Ok(_) => (StatusCode::CREATED, Json(user)).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(user)).into_response(),
    }
}

async fn update_user(
    Path(id): Path<String>,
    Json(payload): Json<UpdateUser>,
    Extension(db): Extension<Arc<RwLock<Database>>>,
) -> impl IntoResponse {
    let collection = db.read().await.collection::<User>("users");
    let object_id = match ObjectId::parse_str(&id) {
        Ok(oid) => oid,
        Err(_) => return (StatusCode::BAD_REQUEST, Json("Invalid ObjectId")).into_response(),
    };
    let filter = doc! { "_id": object_id };
    let update = doc! { "$set": { "username": payload.username } };

    match collection.update_one(filter, update, None).await {
        Ok(update_result) => {
            if update_result.matched_count == 1 {
                (StatusCode::OK, Json("User updated successfully")).into_response()
            } else {
                (StatusCode::NOT_FOUND, Json("User not found")).into_response()
            }
        },
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json("Failed to update user")).into_response(),
    }
}

async fn delete_user(
    Path(id): Path<String>,
    Extension(db): Extension<Arc<RwLock<Database>>>,
) -> impl IntoResponse {
    let collection = db.read().await.collection::<User>("users");
    let object_id = match ObjectId::parse_str(&id) {
        Ok(oid) => oid,
        Err(_) => return (StatusCode::BAD_REQUEST, Json("Invalid ObjectId")).into_response(),
    };
    let filter = doc! { "_id": object_id };

    match collection.delete_one(filter, None).await {
        Ok(delete_result) => {
            if delete_result.deleted_count == 1 {
                (StatusCode::OK, Json("User deleted successfully")).into_response()
            } else {
                (StatusCode::NOT_FOUND, Json("User not found")).into_response()
            }
        },
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json("Failed to delete user")).into_response(),
    }
}

#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

#[derive(Deserialize)]
struct UpdateUser {
    username: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct User {
    #[serde(rename = "_id")]
    id: ObjectId,
    username: String,
}
