# nanohost

## Running Locally (Rust)

This project uses [Diesel](https://diesel.rs/) as ORM. You need to install Diesel CLI to run this project.

```bash
git clone https://github.com/arikchakma/nanohost.git
cd nanohost
cp .env.example .env
diesel migration run
cargo run
```
