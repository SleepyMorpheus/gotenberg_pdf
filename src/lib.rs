#![doc = include_str!("../README.md")]

/// Gotenberg server health status. See [`Client::health_check`].
pub mod health;

mod page_range;
mod paper_format;

pub use crate::paper_format::*;
/// Re-exported from the `bytes` crate (See [`bytes::Bytes`]).
pub use bytes::Bytes;
pub use page_range::*;
use reqwest::multipart;
use reqwest::{Client as ReqwestClient, Error as ReqwestError, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::str::FromStr;

#[cfg(feature = "zeroize")]
use zeroize::Zeroize;

#[cfg(test)]
mod tests;

/// Gotenberg API client.
#[derive(Clone)]
pub struct Client {
    client: ReqwestClient,
    base_url: String,
    username: Option<String>,
    password: Option<String>,
}

impl Drop for Client {
    fn drop(&mut self) {
        // Securely zeroize the username and password
        #[cfg(feature = "zeroize")]
        {
            if let Some(username) = &mut self.username {
                username.zeroize();
            }
            if let Some(password) = &mut self.password {
                password.zeroize();
            }
        }
    }
}

impl Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .field("username", &self.username)
            .finish()
    }
}

impl Client {
    /// Create a new instance of the API client.
    pub fn new(base_url: &str) -> Self {
        // Strip trailing slashes
        let base_url = base_url.trim_end_matches('/');

        Client {
            client: ReqwestClient::new(),
            base_url: base_url.to_string(),
            username: None,
            password: None,
        }
    }

    /// Create a new instance of the API client with basic auth.
    /// You can set the username and password on the Gotenberg server by starting it with `--api-enable-basic-auth` and supplying `GOTENBERG_API_BASIC_AUTH_USERNAME` and `GOTENBERG_API_BASIC_AUTH_PASSWORD` environment variables.
    pub fn new_with_auth(base_url: &str, username: &str, password: &str) -> Self {
        // Strip trailing slashes
        let base_url = base_url.trim_end_matches('/');

        Client {
            client: ReqwestClient::new(),
            base_url: base_url.to_string(),
            username: Some(username.to_string()),
            password: Some(password.to_string()),
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

        // Add basic auth if username and password are provided
        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            req = req.basic_auth(username, Some(password));
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

    /// Convert a URL to a PDF using the Chromium engine.
    pub async fn pdf_from_url(&self, url: &str, options: WebOptions) -> Result<Bytes, Error> {
        let trace = options.trace_id.clone();
        let form = multipart::Form::new().text("url", url.to_string());
        let form = options.fill_form(form);
        self.post("forms/chromium/convert/url", form, trace).await
    }

    /// Convert HTML to a PDF using the Chromium engine.
    pub async fn pdf_from_html(&self, html: &str, options: WebOptions) -> Result<Bytes, Error> {
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

    /// Convert Markdown to a PDF using the Chromium engine.
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
    pub async fn pdf_from_markdown(
        &self,
        html_template: &str,
        markdown: HashMap<&str, &str>,
        options: WebOptions,
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
                    return Err(Error::FilenameError(
                        "Markdown filename must end with '.md'".to_string(),
                    ));
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

    /// Take a screenshot of a webpage using the Chromium engine.
    pub async fn screenshot_url(
        &self,
        url: &str,
        options: ScreenshotOptions,
    ) -> Result<Bytes, Error> {
        let trace = options.trace_id.clone();
        let form = multipart::Form::new().text("url", url.to_string());
        let form = options.fill_form(form);
        self.post("forms/chromium/screenshot/url", form, trace)
            .await
    }

    /// Take a screenshot of an HTML page using the Chromium engine.
    pub async fn screenshot_html(
        &self,
        html: &str,
        options: ScreenshotOptions,
    ) -> Result<Bytes, Error> {
        let trace = options.trace_id.clone();

        let form = multipart::Form::new();
        let file_bytes = html.to_string().into_bytes();
        let part = multipart::Part::bytes(file_bytes)
            .file_name("index.html")
            .mime_str("text/html")
            .unwrap();
        let form = form.part("index.html", part);
        let form = options.fill_form(form);
        self.post("forms/chromium/screenshot/html", form, trace)
            .await
    }

    /// Take a screenshot of a set of markdown files using the Chromium engine.
    pub async fn screenshot_markdown(
        &self,
        html_template: &str,
        markdown: HashMap<&str, &str>,
        options: ScreenshotOptions,
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
                    return Err(Error::FilenameError(
                        "Markdown filename must end with '.md'".to_string(),
                    ));
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

        self.post("forms/chromium/screenshot/markdown", form, trace)
            .await
    }

    /// Convert a document to a PDF using the LibreOffice engine.
    ///
    /// Supports the following file formats:
    /// ```txt
    /// .123 .602 .abw .bib .bmp .cdr .cgm .cmx .csv .cwk .dbf .dif .doc
    /// .docm .docx .dot .dotm .dotx .dxf .emf .eps .epub .fodg .fodp .fods
    /// .fodt .fopd .gif .htm .html .hwp .jpeg .jpg .key .ltx .lwp .mcw .met
    /// .mml .mw .numbers .odd .odg .odm .odp .ods .odt .otg .oth .otp .ots .ott
    /// .pages .pbm .pcd .pct .pcx .pdb .pdf .pgm .png .pot .potm .potx .ppm .pps
    /// .ppt .pptm .pptx .psd .psw .pub .pwp .pxl .ras .rtf .sda .sdc .sdd .sdp .sdw
    /// .sgl .slk .smf .stc .std .sti .stw .svg .svm .swf .sxc .sxd .sxg .sxi .sxm
    /// .sxw .tga .tif .tiff .txt .uof .uop .uos .uot .vdx .vor .vsd .vsdm .vsdx
    /// .wb2 .wk1 .wks .wmf .wpd .wpg .wps .xbm .xhtml .xls .xlsb .xlsm .xlsx .xlt
    /// .xltm .xltx .xlw .xml .xpm .zabw
    /// ```
    pub async fn pdf_from_doc(
        &self,
        filename: &str,
        bytes: Vec<u8>,
        options: DocumentOptions,
    ) -> Result<Bytes, Error> {
        let trace = options.trace_id.clone();

        let form = multipart::Form::new();
        let part = multipart::Part::bytes(bytes).file_name(filename.to_string());
        let form = form.part("files", part);
        let form = options.fill_form(form);
        self.post("forms/libreoffice/convert", form, trace).await
    }

    /// Get the health status of the Gotenberg server.
    pub async fn health_check(&self) -> Result<health::Health, Error> {
        let url = format!("{}/health", self.base_url);
        let response = self.client.get(&url).send().await.map_err(Into::into)?;
        let body = response.text().await.map_err(Into::into)?;
        serde_json::from_str(&body)
            .map_err(|e| Error::ParseError("Health".to_string(), body, e.to_string()))
    }

    /// Get the version of the Gotenberg server.
    pub async fn version(&self) -> Result<String, Error> {
        let url = format!("{}/version", self.base_url);
        let response = self.client.get(&url).send().await.map_err(Into::into)?;
        let body = response.text().await.map_err(Into::into)?;
        Ok(body)
    }

    /// Get the metrics of the Gotenberg server in prometheus format.
    /// The results will not be parsed and are returned as a multi-line string.
    ///
    /// By default the namespace is `gotenberg`, but this can be changed by passing `--prometheus-namespace` to the Gotenberg server on startup.
    ///
    /// - `{namespace}_chromium_requests_queue_size`    Current number of Chromium conversion requests waiting to be treated.
    /// - `{namespace}_chromium_restarts_count`	        Current number of Chromium restarts.
    /// - `{namespace}_libreoffice_requests_queue_size`	Current number of LibreOffice conversion requests waiting to be treated.
    /// - `{namespace}_libreoffice_restarts_count`	    Current number of LibreOffice restarts.
    pub async fn metrics(&self) -> Result<String, Error> {
        let url = format!("{}/prometheus/metrics", self.base_url);
        let response = self.client.get(&url).send().await.map_err(Into::into)?;
        let body = response.text().await.map_err(Into::into)?;
        Ok(body)
    }
}

/// Error type for the Gotenberg API.
#[derive(Debug)]
pub enum Error {
    /// Filename Error
    FilenameError(String),

    /// Error communicating with the gotenberg server.
    CommunicationError(ReqwestError),

    /// PDF rendering error.
    RenderingError(String),

    /// Error parsing a string into a type
    // (Type, Subject, Message)
    ParseError(String, String, String),
}

impl Into<Error> for ReqwestError {
    fn into(self) -> Error {
        Error::CommunicationError(self)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::FilenameError(e) => write!(f, "gotenberg_pdf: Filename Error: {}", e),
            Error::CommunicationError(e) => write!(
                f,
                "gotenberg_pdf: Error communicating with the guotenberg server: {}",
                e
            ),
            Error::RenderingError(e) => {
                write!(f, "gotenberg_pdf: PDF / Image Rendering Error: {}", e)
            }
            Error::ParseError(t, s, e) => {
                write!(f, "gotenberg_pdf: Error Parsing {} from `{}`: {}", t, s, e)
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::CommunicationError(e) => Some(e),
            _ => None,
        }
    }
}

/// Configuration for rendering PDF from web content using the Chromium engine.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebOptions {
    /// By default, the API assigns a unique UUID trace to every request. However, you also have the option to specify the trace for each request.
    /// This trace will show up on the end server as a `Gotenberg-Trace` header.
    pub trace_id: Option<String>,

    /// Define whether to print the entire content on one single page.
    /// Default: `false`
    pub single_page: Option<bool>,

    /// Specify paper width using units like 72pt, 96px, 1in, 25.4mm, 2.54cm, or 6pc.
    /// Default: `8.5` (inches)
    pub paper_width: Option<LinearDimention>,

    /// Specify paper height using units like 72pt, 96px, 1in, 25.4mm, 2.54cm, or 6pc.
    /// Default: `11` (inches)
    pub paper_height: Option<LinearDimention>,

    /// Specify top margin width using units like 72pt, 96px, 1in, 25.4mm, 2.54cm, or 6pc.
    /// Default: `0.39` (inches)
    pub margin_top: Option<LinearDimention>,

    /// Specify bottom margin width using units like 72pt, 96px, 1in, 25.4mm, 2.54cm, or 6pc.
    /// Default: `0.39` (inches)
    pub margin_bottom: Option<LinearDimention>,

    /// Specify left margin width using units like 72pt, 96px, 1in, 25.4mm, 2.54cm, or 6pc.
    /// Default: `0.39` (inches)
    pub margin_left: Option<LinearDimention>,

    /// Specify right margin width using units like 72pt, 96px, 1in, 25.4mm, 2.54cm, or 6pc.
    /// Default: `0.39` (inches)
    pub margin_right: Option<LinearDimention>,

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
    pub native_page_ranges: Option<PageRange>,

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
    pub wait_for_expression: Option<String>,

    /// The media type to emulate, either "screen" or "print". Default: "print".
    pub emulated_media_type: Option<MediaType>,

    /// Cookies to store in the Chromium cookie jar
    pub cookies: Option<Vec<Cookie>>,

    /// Do not wait for Chromium network to be idle. Default: true.
    ///
    /// If you are having problems where the page is not fully rendered, try setting this to false.
    pub skip_network_idle_events: Option<bool>,

    /// Override the default User-Agent HTTP header.
    pub user_agent: Option<String>,

    /// Extra HTTP headers to send by Chromium.
    pub extra_http_headers: Option<HashMap<String, String>>,

    /// Convert the resulting PDF into the given PDF/A format
    pub pdfa: Option<PDFFormat>,

    /// Enable PDF for Universal Access for optimal accessibility.
    pub pdfua: Option<bool>,

    /// Write PDF metadata.
    /// Not all metadata are writable. Consider taking a look at <https://exiftool.org/TagNames/XMP.html#pdf> for an (exhaustive?) list of available metadata.
    /// Caution: Writing metadata may compromise PDF/A compliance.
    pub metadata: Option<HashMap<String, serde_json::Value>>,

    /// Fail on these HTTP status codes.
    /// Fail a response if the HTTP status code from the main page is not acceptable.
    /// An X99 entry means every HTTP status codes between X00 and X99 (e.g., 499 means every HTTP status codes between 400 and 499).
    /// Default: `[499,599]` (all 4XX and 5XX status codes)
    pub fail_on_http_status_codes: Option<Vec<u32>>,

    /// Fail on these HTTP status codes on resources.
    /// Fail a response if any of the resources loaded in the page have a status code that is not acceptable.
    /// An X99 entry means every HTTP status codes between X00 and X99 (e.g., 499 means every HTTP status codes between 400 and 499).
    /// Default: None
    pub fail_on_resource_http_status_codes: Option<Vec<u32>>,

    /// Fail a response if Chromium fails to load at least one resource. Default: `false`.
    pub fail_on_resource_loading_failed: Option<bool>,

    /// Fail a response if there are exceptions in the Chromium console.
    pub fail_on_console_exceptions: Option<bool>,
}

impl WebOptions {
    /// Set the paper format. If a custom paper size is needed, set the `paper_width` and `paper_height` fields manually.
    pub fn set_paper_format(&mut self, format: PaperFormat) {
        self.paper_width = Some(format.width());
        self.paper_height = Some(format.height());
    }

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
            form = form.text("nativePageRanges", native_page_ranges.to_string());
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

        if let Some(wait_for_expression) = self.wait_for_expression {
            form = form.text("waitForExpression", wait_for_expression);
        }

        if let Some(emulated_media_type) = self.emulated_media_type {
            form = form.text("emulatedMediaType", emulated_media_type.to_string());
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

        if let Some(user_agent) = self.user_agent {
            form = form.text("userAgent", user_agent);
        }

        if let Some(extra_http_headers) = self.extra_http_headers {
            form = form.text(
                "extraHttpHeaders",
                serde_json::to_string(&extra_http_headers).unwrap(),
            );
        }

        if let Some(pdfa) = self.pdfa {
            form = form.text("pdfa", pdfa.to_string());
        }

        if let Some(pdfua) = self.pdfua {
            form = form.text("pdfua", pdfua.to_string());
        }

        if let Some(metadata) = self.metadata {
            form = form.text("metadata", serde_json::to_string(&metadata).unwrap());
        }

        if let Some(fail_on_http_status_codes) = self.fail_on_http_status_codes {
            form = form.text(
                "failOnHttpStatusCodes",
                serde_json::to_string(&fail_on_http_status_codes).unwrap(),
            );
        }

        if let Some(fail_on_resource_http_status_codes) = self.fail_on_resource_http_status_codes {
            form = form.text(
                "failOnResourceHttpStatusCodes",
                serde_json::to_string(&fail_on_resource_http_status_codes).unwrap(),
            );
        }

        if let Some(fail_on_resource_loading_failed) = self.fail_on_resource_loading_failed {
            form = form.text(
                "failOnResourceLoadingFailed",
                fail_on_resource_loading_failed.to_string(),
            );
        }

        if let Some(fail_on_console_exceptions) = self.fail_on_console_exceptions {
            form = form.text(
                "failOnConsoleExceptions",
                fail_on_console_exceptions.to_string(),
            );
        }

        form
    }
}

/// Options for taking a screenshot of a webpage.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ScreenshotOptions {
    /// By default, the API assigns a unique UUID trace to every request. However, you also have the option to specify the trace for each request.
    /// This trace will show up on the end server as a `Gotenberg-Trace` header.
    pub trace_id: Option<String>,

    /// The device screen width in pixels. Default: 800.
    pub width: Option<u32>,

    /// The device screen height in pixels. Default: 600.
    pub height: Option<u32>,

    /// Define whether to clip the screenshot according to the device dimensions. Default: false.
    pub clip: Option<bool>,

    /// The image format, either "png", "jpeg" or "webp". Default: png.
    pub format: Option<ImageFormat>,

    /// The compression quality from range 0 to 100 (jpeg only). Default: 100.
    pub quality: Option<u8>,

    /// Hide the default white background and allow generating screenshots with transparency. Default: false.
    pub omit_background: Option<bool>,

    /// Define whether to optimize image encoding for speed, not for resulting size. Default: false.
    pub optimize_for_speed: Option<bool>,

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
    pub wait_for_expression: Option<String>,

    /// The media type to emulate, either "screen" or "print". Default: "print".
    pub emulated_media_type: Option<MediaType>,

    /// Cookies to store in the Chromium cookie jar
    pub cookies: Option<Vec<Cookie>>,

    /// Do not wait for Chromium network to be idle. Default: true.
    ///
    /// If you are having problems where the page is not fully rendered, try setting this to false.
    pub skip_network_idle_events: Option<bool>,

    /// Override the default User-Agent HTTP header.
    pub user_agent: Option<String>,

    /// Extra HTTP headers to send by Chromium.
    pub extra_http_headers: Option<HashMap<String, String>>,

    /// Fail on these HTTP status codes.
    /// Fail a response if the HTTP status code from the main page is not acceptable.
    /// An X99 entry means every HTTP status codes between X00 and X99 (e.g., 499 means every HTTP status codes between 400 and 499).
    /// Default: `[499,599]` (all 4XX and 5XX status codes)
    pub fail_on_http_status_codes: Option<Vec<u32>>,

    /// Fail on these HTTP status codes on resources.
    /// Fail a response if any of the resources loaded in the page have a status code that is not acceptable.
    /// An X99 entry means every HTTP status codes between X00 and X99 (e.g., 499 means every HTTP status codes between 400 and 499).
    /// Default: None
    pub fail_on_resource_http_status_codes: Option<Vec<u32>>,

    /// Fail a response if Chromium fails to load at least one resource. Default: `false`.
    pub fail_on_resource_loading_failed: Option<bool>,

    /// Fail a response if there are exceptions in the Chromium console.
    pub fail_on_console_exceptions: Option<bool>,
}

impl ScreenshotOptions {
    fn fill_form(self, form: reqwest::multipart::Form) -> reqwest::multipart::Form {
        let mut form = form;

        if let Some(width) = self.width {
            form = form.text("width", width.to_string());
        }

        if let Some(height) = self.height {
            form = form.text("height", height.to_string());
        }

        if let Some(clip) = self.clip {
            form = form.text("clip", clip.to_string());
        }

        if let Some(format) = self.format {
            form = form.text("format", format.to_string());
        }

        if let Some(quality) = self.quality {
            form = form.text("quality", quality.to_string());
        }

        if let Some(omit_background) = self.omit_background {
            form = form.text("omitBackground", omit_background.to_string());
        }

        if let Some(optimize_for_speed) = self.optimize_for_speed {
            form = form.text("optimizeForSpeed", optimize_for_speed.to_string());
        }

        if let Some(wait_delay) = self.wait_delay {
            form = form.text("waitDelay", wait_delay);
        }

        if let Some(wait_for_expression) = self.wait_for_expression {
            form = form.text("waitForExpression", wait_for_expression);
        }

        if let Some(emulated_media_type) = self.emulated_media_type {
            form = form.text("emulatedMediaType", emulated_media_type.to_string());
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

        if let Some(user_agent) = self.user_agent {
            form = form.text("userAgent", user_agent);
        }

        if let Some(extra_http_headers) = self.extra_http_headers {
            form = form.text(
                "extraHttpHeaders",
                serde_json::to_string(&extra_http_headers).unwrap(),
            );
        }

        if let Some(fail_on_http_status_codes) = self.fail_on_http_status_codes {
            form = form.text(
                "failOnHttpStatusCodes",
                serde_json::to_string(&fail_on_http_status_codes).unwrap(),
            );
        }

        if let Some(fail_on_resource_http_status_codes) = self.fail_on_resource_http_status_codes {
            form = form.text(
                "failOnResourceHttpStatusCodes",
                serde_json::to_string(&fail_on_resource_http_status_codes).unwrap(),
            );
        }

        if let Some(fail_on_resource_loading_failed) = self.fail_on_resource_loading_failed {
            form = form.text(
                "failOnResourceLoadingFailed",
                fail_on_resource_loading_failed.to_string(),
            );
        }

        if let Some(fail_on_console_exceptions) = self.fail_on_console_exceptions {
            form = form.text(
                "failOnConsoleExceptions",
                fail_on_console_exceptions.to_string(),
            );
        }

        form
    }
}

/// Options for converting a document to a PDF using the LibreOffice engine.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DocumentOptions {
    /// By default, the API assigns a unique UUID trace to every request. However, you also have the option to specify the trace for each request.
    /// This trace will show up on the end server as a `Gotenberg-Trace` header.
    pub trace_id: Option<String>,

    /// Set the password for opening the source file.
    pub password: Option<String>,

    /// Set the paper orientation to landscape. default: false
    pub landscape: Option<bool>,

    /// Page ranges to print, e.g., '1-4' - empty means all pages. default: All pages
    pub native_page_ranges: Option<PageRange>,

    /// Specify whether form fields are exported as widgets or only their fixed print representation is exported. default: true
    pub export_form_fields: Option<bool>,

    /// Specify whether multiple form fields exported are allowed to have the same field name. default: false
    pub allow_duplicate_field_names: Option<bool>,

    /// Specify if bookmarks are exported to PDF. default: true
    pub export_bookmarks: Option<bool>,

    /// Specify that the bookmarks contained in the source LibreOffice file should be exported to the PDF file as Named Destination. default: false
    pub export_bookmarks_to_pdf_destination: Option<bool>,

    /// Export the placeholders fields visual markings only. The exported placeholder is ineffective. default: false
    pub export_placeholders: Option<bool>,

    /// Specify if notes are exported to PDF. default: false
    pub export_notes: Option<bool>,

    /// Specify if notes pages are exported to PDF. Notes pages are available in Impress documents only. default: false
    pub export_notes_pages: Option<bool>,

    /// Specify, if the form field exportNotesPages is set to true, if only notes pages are exported to PDF. default: false
    pub export_only_notes_pages: Option<bool>,

    /// Specify if notes in margin are exported to PDF. default: false
    pub export_notes_in_margin: Option<bool>,

    /// Specify that the target documents with `.od[tpgs]` extension, will have that extension changed to .pdf when the link is exported to PDF. The source document remains untouched. default: false
    pub convert_ooo_target_to_pdf_target: Option<bool>,

    /// Specify that the file system related hyperlinks (file:// protocol) present in the document will be exported as relative to the source document location. default: false
    pub export_links_relative_fsys: Option<bool>,

    /// Export, for LibreOffice Impress, slides that are not included in slide shows. default: false
    pub export_hidden_slides: Option<bool>,

    /// Specify that automatically inserted empty pages are suppressed. This option is active only if storing Writer documents. default: false
    pub skip_empty_pages: Option<bool>,

    /// Specify that a stream is inserted to the PDF file which contains the original document for archiving purposes. default: false
    pub add_original_document_as_stream: Option<bool>,

    /// Ignore each sheet’s paper size, print ranges and shown/hidden status and puts every sheet (even hidden sheets) on exactly one page. default: false
    pub single_page_sheets: Option<bool>,

    /// Specify if images are exported to PDF using a lossless compression format like PNG or compressed using the JPEG format. default: false
    pub lossless_image_compression: Option<bool>,

    /// Specify the quality of the JPG export. A higher value produces a higher-quality image and a larger file. Between 1 and 100. default: 90
    pub quality: Option<u8>,

    /// Specify if the resolution of each image is reduced to the resolution specified by the form field maxImageResolution. default: false
    pub reduce_image_resolution: Option<bool>,

    /// If the form field reduceImageResolution is set to true, tell if all images will be reduced to the given value in DPI. Possible values are: 75, 150, 300, 600 and 1200. default: 300
    pub max_image_resolution: Option<u32>,

    /// Convert the resulting PDF into the given PDF/A format
    pub pdfa: Option<PDFFormat>,

    /// Enable PDF for Universal Access for optimal accessibility.
    pub pdfua: Option<bool>,
}

/// Options for converting a document to a PDF using the LibreOffice engine.
impl DocumentOptions {
    fn fill_form(self, form: reqwest::multipart::Form) -> reqwest::multipart::Form {
        let mut form = form;

        if let Some(password) = self.password {
            form = form.text("password", password);
        }

        if let Some(landscape) = self.landscape {
            form = form.text("landscape", landscape.to_string());
        }

        if let Some(native_page_ranges) = self.native_page_ranges {
            form = form.text("nativePageRanges", native_page_ranges.to_string());
        }

        if let Some(export_form_fields) = self.export_form_fields {
            form = form.text("exportFormFields", export_form_fields.to_string());
        }

        if let Some(allow_duplicate_field_names) = self.allow_duplicate_field_names {
            form = form.text(
                "allowDuplicateFieldNames",
                allow_duplicate_field_names.to_string(),
            );
        }

        if let Some(export_bookmarks) = self.export_bookmarks {
            form = form.text("exportBookmarks", export_bookmarks.to_string());
        }

        if let Some(export_bookmarks_to_pdf_destination) = self.export_bookmarks_to_pdf_destination
        {
            form = form.text(
                "exportBookmarksToPdfDestination",
                export_bookmarks_to_pdf_destination.to_string(),
            );
        }

        if let Some(export_placeholders) = self.export_placeholders {
            form = form.text("exportPlaceholders", export_placeholders.to_string());
        }

        if let Some(export_notes) = self.export_notes {
            form = form.text("exportNotes", export_notes.to_string());
        }

        if let Some(export_notes_pages) = self.export_notes_pages {
            form = form.text("exportNotesPages", export_notes_pages.to_string());
        }

        if let Some(export_only_notes_pages) = self.export_only_notes_pages {
            form = form.text("exportOnlyNotesPages", export_only_notes_pages.to_string());
        }

        if let Some(export_notes_in_margin) = self.export_notes_in_margin {
            form = form.text("exportNotesInMargin", export_notes_in_margin.to_string());
        }

        if let Some(convert_ooo_target_to_pdf_target) = self.convert_ooo_target_to_pdf_target {
            form = form.text(
                "convertOooTargetToPdfTarget",
                convert_ooo_target_to_pdf_target.to_string(),
            );
        }

        if let Some(export_links_relative_fsys) = self.export_links_relative_fsys {
            form = form.text(
                "exportLinksRelativeFsys",
                export_links_relative_fsys.to_string(),
            );
        }

        if let Some(export_hidden_slides) = self.export_hidden_slides {
            form = form.text("exportHiddenSlides", export_hidden_slides.to_string());
        }

        if let Some(skip_empty_pages) = self.skip_empty_pages {
            form = form.text("skipEmptyPages", skip_empty_pages.to_string());
        }

        if let Some(add_original_document_as_stream) = self.add_original_document_as_stream {
            form = form.text(
                "addOriginalDocumentAsStream",
                add_original_document_as_stream.to_string(),
            );
        }

        if let Some(single_page_sheets) = self.single_page_sheets {
            form = form.text("singlePageSheets", single_page_sheets.to_string());
        }

        if let Some(lossless_image_compression) = self.lossless_image_compression {
            form = form.text(
                "losslessImageCompression",
                lossless_image_compression.to_string(),
            );
        }

        if let Some(quality) = self.quality {
            form = form.text("quality", quality.to_string());
        }

        if let Some(reduce_image_resolution) = self.reduce_image_resolution {
            form = form.text("reduceImageResolution", reduce_image_resolution.to_string());
        }

        if let Some(max_image_resolution) = self.max_image_resolution {
            form = form.text("maxImageResolution", max_image_resolution.to_string());
        }

        if let Some(pdfa) = self.pdfa {
            form = form.text("pdfa", pdfa.to_string());
        }

        if let Some(pdfua) = self.pdfua {
            form = form.text("pdfua", pdfua.to_string());
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
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Set the cookie to secure if true.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure: Option<bool>,

    /// Set the cookie as HTTP-only if true.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_only: Option<bool>,

    /// Accepted values are "Strict", "Lax" or "None".
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
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

/// Supported PDF binary formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PDFFormat {
    /// PDF/A-1: (ISO 19005-1:2005)
    #[serde(rename = "PDF/A-1b")]
    A1b,

    /// PDF/A-2: (ISO 19005-2:2011)
    #[serde(rename = "PDF/A-2b")]
    A2b,

    /// PDF/A-3 (ISO 19005-3:2012)
    #[serde(rename = "PDF/A-3b")]
    A3b,
}

impl PDFFormat {
    pub fn to_string(&self) -> String {
        match self {
            PDFFormat::A1b => "PDF/A-1b".to_string(),
            PDFFormat::A2b => "PDF/A-2b".to_string(),
            PDFFormat::A3b => "PDF/A-3b".to_string(),
        }
    }
}

impl fmt::Display for PDFFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl FromStr for PDFFormat {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PDF/A-1b" => Ok(PDFFormat::A1b),
            "PDF/A-2b" => Ok(PDFFormat::A2b),
            "PDF/A-3b" => Ok(PDFFormat::A3b),
            _ => Err(Error::ParseError(
                "PDFFormat".to_string(),
                s.to_string(),
                "Invalid PDF format".to_string(),
            )),
        }
    }
}

/// Image format to use when taking a screenshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageFormat {
    /// Portable Network Graphics (PNG)
    #[serde(rename = "png")]
    Png,

    /// JPEG Image, best for photographs.
    #[serde(rename = "jpeg")]
    Jpeg,

    /// WebP Image, best quality and compression, but not as widely supported.
    #[serde(rename = "webp")]
    Webp,
}

impl ImageFormat {
    pub fn to_string(&self) -> String {
        match self {
            ImageFormat::Png => "png".to_string(),
            ImageFormat::Jpeg => "jpeg".to_string(),
            ImageFormat::Webp => "webp".to_string(),
        }
    }
}

impl fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl FromStr for ImageFormat {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "png" => Ok(ImageFormat::Png),
            "jpeg" => Ok(ImageFormat::Jpeg),
            "webp" => Ok(ImageFormat::Webp),
            _ => Err(Error::ParseError(
                "ImageFormat".to_string(),
                s.to_string(),
                "Invalid image format".to_string(),
            )),
        }
    }
}

/// Media type, either "print" or "screen".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaType {
    #[serde(rename = "screen")]
    /// Screen media type.
    Screen,

    #[serde(rename = "print")]
    /// Print media type.
    Print,
}

impl MediaType {
    pub fn to_string(&self) -> String {
        match self {
            MediaType::Screen => "screen".to_string(),
            MediaType::Print => "print".to_string(),
        }
    }
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl FromStr for MediaType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "screen" => Ok(MediaType::Screen),
            "print" => Ok(MediaType::Print),
            _ => Err(Error::ParseError(
                "MediaType".to_string(),
                s.to_string(),
                "Invalid media type".to_string(),
            )),
        }
    }
}
