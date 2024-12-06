#![doc = include_str!("../README.md")]

mod paper_format;
pub use crate::paper_format::*;
use bytes::Bytes;
use reqwest::multipart;
use reqwest::{Client as ReqwestClient, Error as ReqwestError, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Gotenberg API client.
#[derive(Debug)]
pub struct Client {
    client: ReqwestClient,
    base_url: String,
}

impl Client {
    /// Create a new instance of the API client.
    pub fn new(base_url: &str) -> Self {
        // Strip trailing slashes
        let base_url = base_url.trim_end_matches('/');

        Client {
            client: ReqwestClient::new(),
            base_url: base_url.to_string(),
        }
    }

    /// Generic POST method that takes a multipart form and sends it.
    async fn post(
        &self,
        endpoint: &str,
        form: multipart::Form,
        trace: Option<String>,
    ) -> Result<Bytes, Error> {
        let url = format!("{}/{}", self.base_url, endpoint);

        let mut req = self.client.post(&url).multipart(form);
        if let Some(trace) = trace {
            req = req.header("Gotenberg-Trace", trace);
        }
        let response: Response = req.send().await.map_err(Into::into)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::RenderingError(format!(
                "Failed to render PDF: {} - {}",
                status, body
            )));
        }

        response.bytes().await.map_err(Into::into)
    }

    /// Convert a URL to a PDF.
    pub async fn url_to_pdf(&self, url: &str, options: RequestOptions) -> Result<Bytes, Error> {
        let trace = options.trace_id.clone();
        let form = multipart::Form::new().text("url", url.to_string());
        let form = options.fill_form(form);
        self.post("forms/chromium/convert/url", form, trace).await
    }

    /// Convert HTML to a PDF.
    pub async fn html_to_pdf(&self, html: &str, options: RequestOptions) -> Result<Bytes, Error> {
        let trace = options.trace_id.clone();

        let form = multipart::Form::new();
        let file_bytes = html.to_string().into_bytes();
        let part = multipart::Part::bytes(file_bytes)
            .file_name("index.html")
            .mime_str("text/html")
            .unwrap();
        let form = form.part("index.html", part);
        let form = options.fill_form(form);
        self.post("forms/chromium/convert/html", form, trace).await
    }

    /// Convert Markdown to a PDF.
    ///
    /// The HTML template should in the following format:
    ///
    /// ```html
    /// <!doctype html>
    /// <html lang="en">
    ///  <head>
    ///    <meta charset="utf-8">
    ///    <title>My PDF</title>
    ///  </head>
    ///  <body>
    ///    {{ toHTML "file.md" }}
    ///  </body>
    /// </html>
    /// ```
    ///
    /// The markdown files should be in a "filename" => "content" format. The filename key string must end with `.md`.
    pub async fn markdown_to_pdf(
        &self,
        html_template: &str,
        markdown: HashMap<&str, &str>,
        options: RequestOptions,
    ) -> Result<Bytes, Error> {
        let trace = options.trace_id.clone();

        let form = multipart::Form::new();

        let file_bytes = html_template.to_string().into_bytes();
        let part = multipart::Part::bytes(file_bytes)
            .file_name("index.html")
            .mime_str("text/html")
            .unwrap();
        let form = form.part("index.html", part);
        let form = options.fill_form(form);

        let form = {
            let mut form = form;
            for (filename, content) in markdown {
                if !filename.ends_with(".md") {
                    panic!("Markdown filename must end with '.md'");
                }
                let file_bytes = content.to_string().into_bytes();
                let part = multipart::Part::bytes(file_bytes)
                    .file_name(filename.to_string())
                    .mime_str("text/markdown")
                    .unwrap();
                form = form.part(filename.to_string(), part);
            }
            form
        };

        self.post("forms/chromium/convert/markdown", form, trace)
            .await
    }
}

#[derive(Debug)]
pub enum Error {
    /// Error commmuniction with the guotenberg server.
    CommunicationError(ReqwestError),

    /// PDF rendering error.
    RenderingError(String),
}

impl Into<Error> for ReqwestError {
    fn into(self) -> Error {
        Error::CommunicationError(self)
    }
}

/// Configuration for PDF rendering options.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestOptions {
    /// By default, the API assigns a unique UUID trace to every request. However, you also have the option to specify the trace for each request.
    /// This trace will show up on the end server as a `Gotenberg-Trace` header.
    pub trace_id: Option<String>,

    /// Define whether to print the entire content on one single page.
    /// Default: `false`
    pub single_page: Option<bool>,

    /// Specify paper width using units like 72pt, 96px, 1in, 25.4mm, 2.54cm, or 6pc.
    /// Default: `8.5` (inches)
    pub paper_width: Option<PaperSize>,

    /// Specify paper height using units like 72pt, 96px, 1in, 25.4mm, 2.54cm, or 6pc.
    /// Default: `11` (inches)
    pub paper_height: Option<PaperSize>,

    /// Specify top margin width using units like 72pt, 96px, 1in, 25.4mm, 2.54cm, or 6pc.
    /// Default: `0.39` (inches)
    pub margin_top: Option<f64>,

    /// Specify bottom margin width using units like 72pt, 96px, 1in, 25.4mm, 2.54cm, or 6pc.
    /// Default: `0.39` (inches)
    pub margin_bottom: Option<f64>,

    /// Specify left margin width using units like 72pt, 96px, 1in, 25.4mm, 2.54cm, or 6pc.
    /// Default: `0.39` (inches)
    pub margin_left: Option<f64>,

    /// Specify right margin width using units like 72pt, 96px, 1in, 25.4mm, 2.54cm, or 6pc.
    /// Default: `0.39` (inches)
    pub margin_right: Option<f64>,

    /// Define whether to prefer page size as defined by CSS.
    /// Default: `false`
    pub prefer_css_page_size: Option<bool>,

    /// Define whether the document outline should be embedded into the PDF.
    /// Default: `false`
    pub generate_document_outline: Option<bool>,

    /// Print the background graphics.
    /// Default: `false`
    pub print_background: Option<bool>,

    /// Hide the default white background and allow generating PDFs with transparency.
    /// Default: `false`
    pub omit_background: Option<bool>,

    /// Set the page orientation to landscape.
    /// Default: `false`
    pub landscape: Option<bool>,

    /// The scale of the page rendering.
    /// Default: `1.0`
    pub scale: Option<f64>,

    /// Page ranges to print, e.g., '1-5, 8, 11-13' - empty means all pages.
    /// Default: `All pages`
    pub native_page_ranges: Option<String>,

    /// HTML content containing the header.
    ///
    /// The following classes allow you to inject printing values into the header:
    ///   date - formatted print date.
    ///   title - document title.
    ///   url - document location.
    ///   pageNumber - current page number.
    ///   totalPages - total pages in the document.
    ///
    /// Caveats: No JavaScript or external resources.
    pub header_html: Option<String>,

    /// HTML content containing the footer.
    ///
    /// The following classes allow you to inject printing values into the footer:
    ///   date - formatted print date.
    ///   title - document title.
    ///   url - document location.
    ///   pageNumber - current page number.
    ///   totalPages - total pages in the document.
    ///
    /// Caveats: No JavaScript or external resources.
    pub footer_html: Option<String>,

    /// Duration (e.g, '5s') to wait when loading an HTML document before converting it into PDF.
    pub wait_delay: Option<String>,

    /// The JavaScript expression to wait before converting an HTML document into PDF until it returns true.
    ///
    /// For example:
    ///    ```text
    ///    # Somewhere in the HTML document.
    ///    var globalVar = 'notReady'
    ///    await promises()
    ///    window.globalVar = 'ready'
    ///    ```
    ///
    ///    ```text
    ///    request_options.wait_until = Some("window.globalVar === 'ready'".to_string());
    ///    ```
    pub wait_until: Option<String>,

    /// The media type to emulate, either "screen" or "print". Default: "print".
    pub emulated_media_type: Option<String>,

    /// Cookies to store in the Chromium cookie jar
    pub cookies: Option<Vec<Cookie>>,

    /// Do not wait for Chromium network to be idle. Default: true.
    ///
    /// If you are having problems where the page is not fully rendered, try setting this to false.
    pub skip_network_idle_events: Option<bool>,
    // TODO: userAgent
    // TODO: extraHttpHeaders
    // TODO: failOnHttpStatusCodes
    // TODO: failOnResourceHttpStatusCodes
    // TODO: failOnResourceLoadingFailed
    // TODO: failOnConsoleExceptions
    // TODO: pdfa
    // TODO: pdfua
    // TODO: metadata
}

impl RequestOptions {
    fn fill_form(self, form: reqwest::multipart::Form) -> reqwest::multipart::Form {
        let mut form = form;

        if let Some(single_page) = self.single_page {
            form = form.text("singlePage", single_page.to_string());
        }

        if let Some(paper_width) = self.paper_width {
            form = form.text("paperWidth", format!("{}", paper_width));
        }

        if let Some(paper_height) = self.paper_height {
            form = form.text("paperHeight", format!("{}", paper_height));
        }

        if let Some(margin_top) = self.margin_top {
            form = form.text("marginTop", margin_top.to_string());
        }

        if let Some(margin_bottom) = self.margin_bottom {
            form = form.text("marginBottom", margin_bottom.to_string());
        }

        if let Some(margin_left) = self.margin_left {
            form = form.text("marginLeft", margin_left.to_string());
        }

        if let Some(margin_right) = self.margin_right {
            form = form.text("marginRight", margin_right.to_string());
        }

        if let Some(prefer_css_page_size) = self.prefer_css_page_size {
            form = form.text("preferCssPageSize", prefer_css_page_size.to_string());
        }

        if let Some(generate_document_outline) = self.generate_document_outline {
            form = form.text(
                "generateDocumentOutline",
                generate_document_outline.to_string(),
            );
        }

        if let Some(print_background) = self.print_background {
            form = form.text("printBackground", print_background.to_string());
        }

        if let Some(omit_background) = self.omit_background {
            form = form.text("omitBackground", omit_background.to_string());
        }

        if let Some(landscape) = self.landscape {
            form = form.text("landscape", landscape.to_string());
        }

        if let Some(scale) = self.scale {
            form = form.text("scale", scale.to_string());
        }

        if let Some(native_page_ranges) = self.native_page_ranges {
            form = form.text("nativePageRanges", native_page_ranges);
        }

        if let Some(header_html) = self.header_html {
            let file_bytes = header_html.into_bytes();
            let part = multipart::Part::bytes(file_bytes)
                .file_name("header.html")
                .mime_str("text/html")
                .unwrap();
            form = form.part("header.html", part);
        }

        if let Some(footer_html) = self.footer_html {
            let file_bytes = footer_html.into_bytes();
            let part = multipart::Part::bytes(file_bytes)
                .file_name("footer.html")
                .mime_str("text/html")
                .unwrap();
            form = form.part("footer.html", part);
        }

        if let Some(wait_delay) = self.wait_delay {
            form = form.text("waitDelay", wait_delay);
        }

        if let Some(wait_until) = self.wait_until {
            form = form.text("waitUntil", wait_until);
        }

        if let Some(emulated_media_type) = self.emulated_media_type {
            form = form.text("emulatedMediaType", emulated_media_type);
        }

        if let Some(cookies) = self.cookies {
            form = form.text("cookies", serde_json::to_string(&cookies).unwrap());
        }

        if let Some(skip_network_idle_events) = self.skip_network_idle_events {
            form = form.text(
                "skipNetworkIdleEvents",
                skip_network_idle_events.to_string(),
            );
        }

        form
    }
}

/// Cookie to send to the end server.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Cookie {
    /// Cookie name.
    pub name: String,

    /// Cookie value.
    pub value: String,

    /// Cookie domain.
    pub domain: String,

    /// Cookie path.
    pub path: Option<String>,

    /// Set the cookie to secure if true.
    pub secure: Option<bool>,

    /// Set the cookie as HTTP-only if true.
    pub http_only: Option<bool>,

    /// Accepted values are "Strict", "Lax" or "None".
    pub same_site: Option<String>,
}

impl Cookie {
    pub fn new(name: &str, value: &str, domain: &str) -> Self {
        Cookie {
            name: name.to_string(),
            value: value.to_string(),
            domain: domain.to_string(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_url_to_pdf() {
        // Create the API client
        let client = Client::new("http://localhost:3000");

        let mut options = RequestOptions::default();
        options.skip_network_idle_events = Some(false);

        // Call the API and handle the result
        match client.url_to_pdf("https://ocudigital.com", options).await {
            Ok(bytes) => {
                // Verify the response content
                assert!(!bytes.is_empty(), "PDF content should not be empty");
                println!("Received PDF content: {} bytes", bytes.len());

                // Save to local temp directory
                let temp_dir = std::env::temp_dir();
                let pdf_path = temp_dir.join("ocudigital.pdf");
                std::fs::write(&pdf_path, bytes).unwrap();
                println!("PDF saved to: {:?}", pdf_path);
            }
            Err(err) => {
                panic!("API call failed: {:?}", err);
            }
        }
    }
}
