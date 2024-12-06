# Gotenberg PDF Client

**`gotenberg_pdf`** is a Rust library that provides an easy-to-use interface for interacting with the [Gotenberg API](https://gotenberg.dev/). Use this library to convert URLs, HTML, or Markdown to PDF with highly customizable options.

## Features

- **URL to PDF**: Generate PDFs directly from a webpage URL.
- **HTML to PDF**: Convert raw HTML into a PDF.
- **Markdown to PDF**: Render Markdown files into a PDF using an HTML template.
- **Customizable Options**: Configure PDF rendering with paper size, margins, headers/footers, and more.

## Installation

Add `gotenberg_pdf` to your `Cargo.toml`:

```toml
[dependencies]
gotenberg_pdf = "0.1.0"  # Replace with the latest version
```

Ensure you have a running instance of Gotenberg, typically via Docker:

```sh
docker run --rm -p 3000:3000 gotenberg/gotenberg:8
```

## Usage Examples

### Convert URL to PDF

```rust
use gotenberg_pdf::{Client, RequestOptions};
use tokio;

#[tokio::main]
async fn main() {
    // Initialize the client with the Gotenberg server URL
    let client = Client::new("http://localhost:3000");

    // Define optional rendering configurations
    let mut options = RequestOptions::default();
    options.paper_width = Some(8.5); // 8.5 inches
    options.paper_height = Some(11.0); // 11 inches

    // Convert a URL to PDF
    match client.url_to_pdf("https://example.com", options).await {
        Ok(pdf_bytes) => {
            // Save the PDF locally
            std::fs::write("example.pdf", pdf_bytes).expect("Failed to save PDF");
            println!("PDF successfully saved as 'example.pdf'");
        }
        Err(e) => eprintln!("Error creating PDF: {:?}", e),
    }
}
```

### Convert HTML to PDF

```rust
use gotenberg_pdf::{Client, RequestOptions};
use tokio;

#[tokio::main]
async fn main() {
    let client = Client::new("http://localhost:3000");

    let html_content = r#"
    <!doctype html>
    <html>
        <head><title>My PDF</title></head>
        <body><h1>Hello, PDF!</h1></body>
    </html>
    "#;

    let options = RequestOptions::default();

    match client.html_to_pdf(html_content, options).await {
        Ok(pdf_bytes) => {
            std::fs::write("hello.pdf", pdf_bytes).expect("Failed to save PDF");
            println!("PDF successfully saved as 'hello.pdf'");
        }
        Err(e) => eprintln!("Error creating PDF: {:?}", e),
    }
}
```

### Convert Markdown to PDF

```rust
use gotenberg_pdf::{Client, RequestOptions};
use std::collections::HashMap;
use tokio;

#[tokio::main]
async fn main() {
    let client = Client::new("http://localhost:3000");

    // Markdown content
    let mut markdown_files = HashMap::new();
    markdown_files.insert("example.md", "# My Markdown PDF\nThis is a test document.");

    // HTML template to wrap the markdown
    let html_template = r#"
    <!doctype html>
    <html>
        <head><title>Markdown PDF</title></head>
        <body>{{ toHTML "example.md" }}</body>
    </html>
    "#;

    let options = RequestOptions::default();

    match client.markdown_to_pdf(html_template, markdown_files, options).await {
        Ok(pdf_bytes) => {
            std::fs::write("markdown.pdf", pdf_bytes).expect("Failed to save PDF");
            println!("PDF successfully saved as 'markdown.pdf'");
        }
        Err(e) => eprintln!("Error creating PDF: {:?}", e),
    }
}
```

## Configuration Options

`RequestOptions` provides fine-grained control over the PDF generation process. Here are some of the available fields:

- `single_page`: Render all content on a single page. Default is `false`.
- `paper_width` / `paper_height`: Set the paper size in inches. Defaults are `8.5` inches wide and `11` inches tall (standard letter size).
- `margin_top`, `margin_bottom`, `margin_left`, `margin_right`: Configure page margins. Defaults are `0.39` inches on all sides.
- `header_html`, `footer_html`: Add headers and footers to your PDF. These can include placeholders such as:
  - `{{date}}`: Inserts the formatted print date.
  - `{{title}}`: Inserts the document title.
  - `{{url}}`: Inserts the document location.
  - `{{pageNumber}}`: Inserts the current page number.
  - `{{totalPages}}`: Inserts the total number of pages.
- `print_background`: Enable background graphics in the PDF. Default is `false`.
- `landscape`: Set the page orientation to landscape. Default is `false`.
- `prefer_css_page_size`: Use the page size defined by CSS instead of the provided paper dimensions. Default is `false`.
- `generate_document_outline`: Embed the document outline (table of contents) into the PDF. Default is `false`.
- `omit_background`: Hide the default white background and generate a PDF with transparency. Default is `false`.
- `scale`: Set the rendering scale. Defaults to `1.0`.
- `native_page_ranges`: Specify a subset of pages to include in the output PDF (e.g., `1-5, 8, 11-13`). Default is all pages.
- `wait_delay`: Wait for a specified duration (e.g., `5s`) before converting an HTML document to PDF.
- `wait_until`: Provide a JavaScript condition to wait for before rendering (e.g., `window.isReady === true`).
- `emulated_media_type`: Specify the media type to emulate (`"screen"` or `"print"`). Default is `"print"`.
- `cookies`: Add a list of cookies to be used in the request.
- `skip_network_idle_events`: Set to `false` to wait for the network to be fully idle before rendering the PDF. Default is `true`.
