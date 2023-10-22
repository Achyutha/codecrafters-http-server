use std::{fmt::Display, str::from_utf8, collections::HashMap};
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncWriteExt, AsyncReadExt}};

#[derive(Debug, PartialEq)]
enum Verb {
    Get
}

impl Display for Verb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Verb::Get => write!(f, "GET")
        }
    }
}

impl TryFrom<&str> for Verb {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "GET" => Ok(Verb::Get),
            _ => Err(anyhow::anyhow!(format!("Unable to parse the Http Verb {:?}", value)))
        }
    }
}

#[derive(Debug)]
struct HttpRequest {
    path: String,
    verb: Verb,
    headers: HashMap<String, String>
}

impl Display for HttpRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = format!("------------\nREQUEST:\nHTTP: {}\nPATH: {}\nHEADERS:\n{:#?}\n------------", self.verb, self.path, self.headers);
        write!(f, "{}", data)
    }
}

impl TryFrom<Vec<&str>> for HttpRequest {
    type Error = anyhow::Error;

    fn try_from(value: Vec<&str>) -> Result<Self, Self::Error> {
        let payload = value[0];
        let stripped = payload.strip_suffix("HTTP/1.1");
        let mut headers = HashMap::new();

        for header in value.into_iter().skip(1) {
            let parts: Vec<_> = header.split(": ").collect();
            if parts.len() != 2 {
                return Err(anyhow::anyhow!("Unable to parse the HTTP Header - {:?}", parts));
            }
            headers.insert(parts[0].to_string(), parts[1].to_string());
        }

        if stripped.is_none() {
            return Err(anyhow::anyhow!("Protocol doesn't match"))
        }
        let value = stripped.unwrap().trim().to_string();
        let components: Vec<&str> = value.splitn(2, ' ').collect();

        if components.len() != 2 {
            return Err(anyhow::anyhow!("Something went wrong parsing the header components: {:?}", components))
        }

        let verb = Verb::try_from(components[0])?;

        let path = components[1].to_string();

        Ok(HttpRequest {verb, path, headers})
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221").await.unwrap();
    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        process_stream(&mut socket).await;
    }
}

fn process_request(data: &str) -> String {
    let raw_request: Vec<_> = data.split("\r\n").filter(|x| !x.is_empty()).collect();

    if raw_request.is_empty() {
        eprint!("Unable to parse the payload: {:?}", data);
    }

    let request = HttpRequest::try_from(raw_request).unwrap();

    // NOTE: The code only supports HTTP::Get for now!
    if request.verb != Verb::Get {
        return "HTTP/1.1 404 Not Found\r\n\r\n404 Not Found".to_string();
    }

    if request.path.starts_with("/echo/") {
        let data = request.path.strip_prefix("/echo/").unwrap();
        format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", data.len(), data.clone())
    } else if request.path == "/user-agent" {
        let user_agent = request.headers.get("User-Agent").unwrap();
        format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", user_agent.len(), user_agent.clone())
    } else if request.path == "/" {
        "HTTP/1.1 200 OK\r\n\r\n200 OK".to_string()
    } else {
        "HTTP/1.1 404 Not Found\r\n\r\n404 Not Found".to_string()
    }
}

async fn process_stream(stream: &mut TcpStream) {
    let mut buff = [0; 4096];
    let buff_len = stream.read(&mut buff).await.unwrap();
    let data = from_utf8(&buff[..buff_len]).unwrap();
    let response = process_request(data);

    stream.write(response.as_bytes()).await.unwrap();
}
