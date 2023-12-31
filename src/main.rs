use std::{fmt::Display, str::from_utf8, collections::HashMap, env, path::PathBuf};
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncWriteExt, AsyncReadExt}, fs::File};

#[derive(Debug, PartialEq)]
enum Verb {
    Get,
    Post
}

impl Display for Verb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Verb::Get => write!(f, "GET"),
            Verb::Post => write!(f, "POST")
        }
    }
}

impl TryFrom<&str> for Verb {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "GET" => Ok(Verb::Get),
            "POST" => Ok(Verb::Post),
            _ => Err(anyhow::anyhow!(format!("Unable to parse the Http Verb {:?}", value)))
        }
    }
}

#[derive(Debug)]
struct HttpRequest {
    path: String,
    verb: Verb,
    headers: HashMap<String, String>,
    body: Option<String>
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
        let mut headers = HashMap::new();
        let mut body = None;

        let payload = value[0];
        let stripped = payload.strip_suffix("HTTP/1.1");

        if stripped.is_none() {
            return Err(anyhow::anyhow!("Protocol doesn't match"))
        }

        for row in value.clone().into_iter().filter(|x| x.contains(":")).skip(1) {
            let parts: Vec<_> = row.split(": ").collect();
            if parts.len() == 2 {
                headers.insert(parts[0].to_string(), parts[1].to_string());
            }
        }

        if !value.last().unwrap().contains(": ") {
            body = Some(value.last().unwrap().to_string());
        }

        let value = stripped.unwrap().trim().to_string();
        let components: Vec<&str> = value.splitn(2, ' ').collect();
        if components.len() != 2 {
            return Err(anyhow::anyhow!("Something went wrong parsing the header components: {:?}", components))
        }

        let verb = Verb::try_from(components[0])?;

        let path = components[1].to_string();

        Ok(HttpRequest {verb, path, headers, body})
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<_> = env::args().collect();
    let dir = if args.len() > 2 {
        assert_eq!(args[1].as_str(), "--directory");
        Some(args[2].clone())
    } else {
        None
    };
    let listener = TcpListener::bind("127.0.0.1:4221").await.unwrap();
    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        let dir = dir.clone();
        tokio::spawn(async move {
            process_stream(&mut socket, dir).await;
        });
    }
}

async fn process_request(data: &str, dir: Option<String>) -> String {
    let raw_request: Vec<_> = data.split("\r\n").filter(|x| !x.is_empty()).collect();

    if raw_request.is_empty() {
        eprintln!("Unable to parse the payload: {:?}", data);
    }

    let request = HttpRequest::try_from(raw_request).unwrap();

    match (request.verb, request.path.as_str()) {
        (Verb::Get, "/") => {
            "HTTP/1.1 200 OK\r\n\r\n200 OK".to_string()
        },
        (Verb::Get, "/user-agent") => {
            let user_agent = request.headers.get("User-Agent").unwrap();
            format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", user_agent.len(), user_agent.clone())
        },
        (Verb::Get, echo_path) if echo_path.starts_with("/echo/") => {
            let data = request.path.strip_prefix("/echo/").unwrap();
            format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", data.len(), data.clone())
        }
        (Verb::Get, file_path) if file_path.starts_with("/files") => {
            let file_name = request.path.strip_prefix("/files/").unwrap();
            if let Some(dir) = dir {
                let file_path = PathBuf::from_iter([dir.as_str(), file_name].iter());
                match File::open(file_path).await {
                    Ok(mut file) => {
                        let mut buff = String::new();
                        file.read_to_string(&mut buff).await.unwrap();
                        format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}", buff.len(), buff.clone())
                    },
                        Err(_) => {
                        "HTTP/1.1 404 Not Found\r\n\r\n404 Not Found".to_string()
                    }
                }
            } else {
                "HTTP/1.1 404 Not Found\r\n\r\n404 Not Found".to_string()
            }
        }
        (Verb::Post, file_path) if file_path.starts_with("/files") => {
            let file_name = request.path.strip_prefix("/files/").unwrap();
            if let Some(dir) = dir {
                let file_path = PathBuf::from_iter([dir.as_str(), file_name]);
                let mut file = File::create(file_path).await.unwrap();
                _ = file.write_all(request.body.unwrap().as_bytes()).await;
                "HTTP/1.1 201 Created\r\n\r\n".to_string()
            } else {
                "HTTP/1.1 404 Not Found\r\n\r\n404 Not Found".to_string()
            }
        }
        _ => {
            "HTTP/1.1 404 Not Found\r\n\r\n404 Not Found".to_string()
        }
    }
}

async fn process_stream(stream: &mut TcpStream, dir: Option<String>) {
    let mut buff = [0; 4096];
    let buff_len = stream.read(&mut buff).await.unwrap();
    let data = from_utf8(&buff[..buff_len]).unwrap();
    let response = process_request(data, dir).await;

    stream.write(response.as_bytes()).await.unwrap();
}
