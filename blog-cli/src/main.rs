use std::fs;

use blog_client::{AuthToken, PostPatch, Transport};
use clap::{Parser, Subcommand};

const TOKEN_FILE: &str = ".blog_token";

#[derive(Parser)]
#[command(name = "blog-cli", about = "CLI client for blog service")]
struct Cli {
    /// Use gRPC transport instead of HTTP
    #[arg(long, global = true)]
    grpc: bool,

    /// Server address (host:port)
    #[arg(long, global = true)]
    server: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Register a new user
    Register {
        #[arg(long)]
        username: String,
        #[arg(long)]
        email: String,
        #[arg(long)]
        password: String,
    },
    /// Login as existing user
    Login {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password: String,
    },
    /// Create a new post
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        content: String,
    },
    /// Get a post by ID
    Get {
        #[arg(long)]
        id: i64,
    },
    /// Update a post by ID
    Update {
        #[arg(long)]
        id: i64,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        content: Option<String>,
    },
    /// Delete a post by ID
    Delete {
        #[arg(long)]
        id: i64,
    },
    /// List posts
    List {
        #[arg(long, default_value_t = 10)]
        limit: i64,
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },
}

fn load_token() -> Option<AuthToken> {
    fs::read_to_string(TOKEN_FILE)
        .ok()
        .map(|s| AuthToken(s.trim().to_string()))
}

fn save_token(token: &AuthToken) -> std::io::Result<()> {
    fs::write(TOKEN_FILE, &token.0)
}

fn print_post(post: &blog_client::Post) {
    println!("ID:         {}", post.id);
    println!("Author ID:  {}", post.author_id);
    println!("Title:      {}", post.title);
    println!("Content:    {}", post.content);
    println!(
        "Created at: {}",
        post.created_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!(
        "Updated at: {}",
        post.updated_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let transport = if cli.grpc {
        let addr = cli
            .server
            .unwrap_or_else(|| "http://localhost:50051".to_string());
        Transport::Grpc { endpoint: addr }
    } else {
        let addr = cli
            .server
            .unwrap_or_else(|| "http://localhost:3000".to_string());
        Transport::Http { base_url: addr }
    };

    let client = blog_client::connect(transport).await?;
    let token = load_token();

    match cli.command {
        Commands::Register {
            username,
            email,
            password,
        } => {
            let (tok, user_id) = client.register(&username, &email, &password).await?;
            save_token(&tok)?;
            println!("Registered successfully. User ID: {}", user_id);
            println!("Token saved to {}", TOKEN_FILE);
        }
        Commands::Login { username, password } => {
            let tok = client.login(&username, &password).await?;
            save_token(&tok)?;
            println!("Logged in successfully.");
            println!("Token saved to {}", TOKEN_FILE);
        }
        Commands::Create { title, content } => {
            let tok = token.ok_or("Not authenticated. Run `login` first.")?;
            let post = client.create_post(&tok, &title, &content).await?;
            println!("Post created:");
            print_post(&post);
        }
        Commands::Get { id } => {
            let post = client.get_post(id).await?;
            print_post(&post);
        }
        Commands::Update { id, title, content } => {
            let tok = token.ok_or("Not authenticated. Run `login` first.")?;
            let patch = PostPatch { title, content };
            let post = client.update_post(&tok, id, patch).await?;
            println!("Post updated:");
            print_post(&post);
        }
        Commands::Delete { id } => {
            let tok = token.ok_or("Not authenticated. Run `login` first.")?;
            client.delete_post(&tok, id).await?;
            println!("Post {} deleted.", id);
        }
        Commands::List { limit, offset } => {
            let page = client.list_posts(Some(limit), Some(offset)).await?;
            println!(
                "Posts {}-{} of {} total:",
                offset + 1,
                offset + page.posts.len() as i64,
                page.total
            );
            for post in &page.posts {
                println!("---");
                print_post(post);
            }
            if page.posts.is_empty() {
                println!("No posts found.");
            }
        }
    }

    Ok(())
}
