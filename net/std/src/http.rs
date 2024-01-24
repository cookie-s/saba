use dns_lookup::lookup_host;
use std::io::prelude::*;
use std::io::Read;
use std::net::TcpStream;
use std::string::String;
use std::vec::Vec;
use toybr_core::http::{Header, HttpResponse};

pub struct HttpClient {}

impl HttpClient {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get(&self, host: String, port: u16, path: String) -> std::io::Result<HttpResponse> {
        let ips = lookup_host(&host)?.into_iter();
        let ipv4s: Vec<std::net::IpAddr> = ips.filter(|ip| ip.is_ipv4()).collect();

        let mut stream = TcpStream::connect((ipv4s[0], port))?;

        let mut request = String::from("GET /");
        request.push_str(&path);
        request.push_str(" HTTP/1.1\n");

        // headers
        request.push_str("Host: ");
        request.push_str(&host);
        request.push('\n');
        request.push_str("Accept: */*\n");
        request.push_str("Connection: close\n");

        request.push('\n');

        stream.write(request.as_bytes())?;

        let mut buf = String::new();
        stream.read_to_string(&mut buf)?;

        Ok(parse_http_response(buf))
    }

    // TODO: support correctly
    /*
        pub fn _post(&self, url: &ParsedUrl, _body: String) -> std::io::Result<HttpResponse> {
            let ips: Vec<std::net::IpAddr> = lookup_host(&url.host)?;

            let mut stream = TcpStream::connect((ips[0], url.port))?;

            let mut request = String::from("POST ");
            request.push_str(&url.path);
            request.push_str(" HTTP/1.1\n");

            /*
            // headers
            for h in &url.headers {
                request.push_str(&h.key);
                request.push_str(": ");
                request.push_str(&h.value);
                request.push('\n');
            }
            */

            request.push('\n');

            stream.write(request.as_bytes())?;

            let mut buf = String::new();
            stream.read_to_string(&mut buf)?;

            Ok(HttpResponse::new(buf))
        }
    */
}

fn parse_http_response(raw_response: String) -> HttpResponse {
    let preprocessed_response = raw_response.replace("\n\r", "\n");

    let (status_line, remaining) = match preprocessed_response.split_once("\n") {
        Some((s, r)) => (s, r),
        None => panic!("http response doesn't have a new line"),
    };

    let (headers, body) = match remaining.split_once("\n\n") {
        Some((h, b)) => {
            let mut headers = Vec::new();
            for header in h.split("\n") {
                // TODO: remove a new line cleaned_header
                let cleaned_header = header.replace("\r", "");
                let splitted_header: Vec<&str> = cleaned_header.splitn(2, ":").collect();

                headers.push(Header::new(
                    String::from(splitted_header[0]),
                    // TODO: remove a whitespace correctly
                    String::from(splitted_header[1].replacen(" ", "", 1)),
                ));
            }
            (headers, b)
        }
        None => (Vec::new(), remaining),
    };

    let statuses: Vec<&str> = status_line.split(" ").collect();

    HttpResponse::new(
        statuses[0].to_string(),
        match statuses[1].parse() {
            Ok(s) => s,
            Err(_) => 404,
        },
        statuses[2].to_string(),
        headers,
        body.to_string(),
    )
}
