# drupal-rust

Drupal 4.7.0 core functionality ported to Rust. This is a **partial port** only, it does not aim to be feature complete.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- MySQL Database

## Installation

1.  Clone the repository:
    ```bash
    git clone <repository-url>
    cd drupal-rust
    ```

2.  Setup configuration:
    ```bash
    cp .env.example .env
    ```

3.  Edit `.env` and set your database credentials. Ensure the database listed in `DRUPAL_DATABASE__URL` exists (e.g., create it via `CREATE DATABASE drupal;`).

## Running the App

1.  Start the server:
    ```bash
    cargo run
    ```

2.  Navigate to the installation page in your browser:
    http://localhost:8080/install

3.  Follow the on-screen instructions to set up the database and create an admin account.
