use axum::{extract, http};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Serialize, FromRow)]
pub struct Quote {
    id: uuid::Uuid,
    book: String,
    quote: String,
    inserted_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl Quote {
    fn new(book: String, quote: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            book,
            quote,
            inserted_at: now,
            updated_at: now,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct CreateQuote {
    book: String,
    quote: String,
}

pub async fn health() -> http::StatusCode {
    http::StatusCode::OK
}

pub async fn create_quote(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<CreateQuote>,
) -> Result<(http::StatusCode, axum::Json<Quote>), http::StatusCode> {
    let quote = Quote::new(payload.book, payload.quote);
    let res = sqlx::query(
        r#"
        INSERT INTO quotes (id, book, quote, inserted_at, updated_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(quote.id)
    .bind(&quote.book)
    .bind(&quote.quote)
    .bind(quote.inserted_at)
    .bind(quote.updated_at)
    .execute(&pool)
    .await;

    match res {
        Ok(_) => Ok((http::StatusCode::CREATED, axum::Json(quote))),
        Err(_) => Err(http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn read_quotes(
    extract::State(pool): extract::State<PgPool>,
) -> Result<axum::Json<Vec<Quote>>, http::StatusCode> {
    let res = sqlx::query_as::<_, Quote>("SELECT * FROM quotes")
        .fetch_all(&pool)
        .await;
    match res {
        Ok(quotes) => Ok(axum::Json(quotes)),
        Err(_) => Err(http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn update_quote(
    extract::State(pool): extract::State<PgPool>,
    extract::Path(id): extract::Path<uuid::Uuid>,
    axum::Json(payload): axum::Json<CreateQuote>,
) -> http::StatusCode {
    let now = chrono::Utc::now();
    let res = sqlx::query(
        r#"
        UPDATE quotes
        SET book = $1, quote = $2, updated_at = $3
        WHERE id = $4
        "#,
    )
    .bind(&payload.book)
    .bind(&payload.quote)
    .bind(now)
    .bind(id)
    .execute(&pool)
    .await
    .map(|res| match res.rows_affected() {
        0 => http::StatusCode::NOT_FOUND,
        _ => http::StatusCode::OK,
    });
    match res {
        Ok(status) => status,
        Err(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub async fn delete_quote(
    extract::State(pool): extract::State<PgPool>,
    extract::Path(id): extract::Path<uuid::Uuid>,
) -> http::StatusCode {
    let res = sqlx::query("DELETE FROM quotes WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .map(|res| match res.rows_affected() {
            0 => http::StatusCode::NOT_FOUND,
            _ => http::StatusCode::OK,
        });
    match res {
        Ok(status) => status,
        Err(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[sqlx::test(fixtures("quotes"))]
async fn test_create_quote(pool: PgPool) -> sqlx::Result<()> {
    let quote = Quote::new("book".to_string(), "quote".to_string());
    let res = create_quote(
        extract::State(pool),
        axum::Json(CreateQuote {
            book: quote.book.clone(),
            quote: quote.quote.clone(),
        }),
    )
    .await;
    assert!(res.is_ok());
    Ok(())
}

#[sqlx::test(fixtures("quotes"))]
async fn test_read_quotes(pool: PgPool) -> sqlx::Result<()> {
    let res = read_quotes(extract::State(pool)).await;
    assert!(res.is_ok());
    let quotes = res.unwrap();
    assert_eq!(quotes.0.len(), 1);
    // The result contains one quote with id a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11
    assert_eq!(
        quotes.0[0].id,
        uuid::Uuid::parse_str("a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11").unwrap()
    );
    Ok(())
}

#[sqlx::test(fixtures("quotes"))]
async fn test_update_quotes(pool: PgPool) -> sqlx::Result<()> {
    let res = update_quote(
        extract::State(pool.clone()),
        extract::Path(uuid::Uuid::parse_str("a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11").unwrap()),
        axum::Json(CreateQuote {
            book: "book".to_string(),
            quote: "quote".to_string(),
        }),
    )
    .await;
    assert_eq!(res, http::StatusCode::OK);
    // verify that the quote was updated
    let res = read_quotes(extract::State(pool)).await;
    assert!(res.is_ok());
    let quotes = res.unwrap();
    assert_eq!(quotes.0.len(), 1);
    assert_eq!(quotes.0[0].book, "book");
    Ok(())
}

#[sqlx::test(fixtures("quotes"))]
async fn test_delete_quote(pool: PgPool) -> sqlx::Result<()> {
    let res = delete_quote(
        extract::State(pool.clone()),
        extract::Path(uuid::Uuid::parse_str("a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11").unwrap()),
    )
    .await;
    assert_eq!(res, http::StatusCode::OK);
    // verify that the quote was deleted
    let res = read_quotes(extract::State(pool)).await;
    assert!(res.is_ok());
    let quotes = res.unwrap();
    assert_eq!(quotes.0.len(), 0);
    Ok(())
}
