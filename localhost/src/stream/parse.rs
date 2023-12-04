
use std::str;
use std::error::Error;
use http::{Request, Method, Uri, Version, HeaderMap, HeaderValue, HeaderName};

/// Function to parse a raw HTTP request from a Vec<u8> buffer into an http::Request
pub fn parse_raw_request(headers_buffer: Vec<u8>, body_buffer: Vec<u8>) -> Result<Request<Vec<u8>>, Box<dyn Error>> {
  
  if headers_buffer.is_empty() {
    eprintln!("parse_raw_request: headers_buffer is empty");
    return Err(ERROR_400_HEADERS_BUFFER_IS_EMPTY.into());
  }
  
  let headers_string = match String::from_utf8(headers_buffer.clone()){
    Ok(v) => v,
    Err(e) => {
      eprintln!("Failed to convert headers_buffer to string:\n {}", e);
      return Err(ERROR_400_HEADERS_BUFFER_TO_STRING.into())
    }
  };
  
  // Split the request string into lines
  // let mut lines = request_str.lines(); //todo: never use this crap. it is dead for approach more complex than hello\nworld
  
  // separate raw request to ... pieces as vector
  let mut headers_lines: Vec<String> = Vec::new();
  for line in headers_string.split('\n'){ headers_lines.push(line.to_string()); }
  
  if headers_lines.is_empty() {
    eprintln!("headers_lines is empty");
    return Err(ERROR_400_HEADERS_LINES_IS_EMPTY.into())
  }

  // Initialize a new HeaderMap to store the HTTP headers
  let mut headers = HeaderMap::new();
  
  // Parse the request line, which must be the first one
  let request_line: String = match headers_lines.get(0) {
    Some(value) => {value.to_string()},
    None => {
      eprintln!("Fail to get request_line");
      return Err(ERROR_500_INTERNAL_SERVER_ERROR.into())
    },
  };
  
  let (method, uri, version) = match parse_request_line(request_line.clone()){
    Ok(v) => v,
    Err(e) => {
      eprintln!("Failed to parse request_line: {}", e);
      return Err(ERROR_400_HEADERS_FAILED_TO_PARSE.into())
    }
  };
  
  // Parse the headers
  for line_index in 1..headers_lines.len() {
    // global_index += 1;
    let line: String = match headers_lines.get(line_index){
      Some(value) => {value.to_string()},
      None => {
        eprintln!("Fail to get header line");
        return Err(ERROR_500_INTERNAL_SERVER_ERROR.into())
      },
    };
    
    if line.is_empty() { break } //expect this can be the end of headers section
    
    let parts: Vec<String> = line.splitn(2, ": ").map(|s| s.to_string()).collect();
    if parts.len() == 2 {
      let header_name = match HeaderName::from_bytes(parts[0].as_bytes()) {
        Ok(v) => v,
        Err(e) =>{
          eprintln!("Invalid header name: {}\n {}", parts[0], e);
          return Err(ERROR_400_HEADERS_INVALID_HEADER_NAME.into())
        },
      };
      // println!("parsed header_name: {}", header_name); //todo: remove dev print
      // println!("raw header value parts[1]: {}", parts[1]);
      // println!("raw header value len: {}", parts[1].len());
      let value = HeaderValue::from_str( parts[1].trim());
      match value {
        Ok(v) => headers.insert(header_name, v),
        Err(e) =>{
          eprintln!("Invalid header value: {}\n {}", parts[1], e);
          return Err(ERROR_400_HEADERS_INVALID_HEADER_VALUE.into())
        },
      };
      
    }
  }
  
  // Construct the http::Request object
  let mut request = match Request::builder()
  .method(method)
  .uri(uri)
  .version(version)
  .body(body_buffer){
    Ok(v) => v,
    Err(e) => {
      eprintln!("Failed to construct the http::Request object: {}", e);
      return Err(ERROR_500_INTERNAL_SERVER_ERROR.into())
    }
  };
  
  // try to fill the headers, because in builder it looks like there is no method
  // to create headers from HeaderMap, but maybe force replacement can be used too
  let request_headers = request.headers_mut();
  // request_headers.clear();//todo: not safe, maybe some default must present
  for (key,value) in headers{
    let header_name = match key {
      Some(v) => v,
      None => {
        eprintln!("Invalid header name");
        return Err(ERROR_400_HEADERS_INVALID_HEADER_NAME.into())
      },
    };
    
    request_headers.append(header_name, value);
  }
  
  Ok(request)
  
}

use std::str::FromStr;

use crate::stream::errors::{ERROR_400_HEADERS_INVALID_REQUEST_LINE, ERROR_400_HEADERS_INVALID_METHOD, ERROR_400_HEADERS_INVALID_VERSION, ERROR_400_HEADERS_BUFFER_IS_EMPTY, ERROR_400_HEADERS_BUFFER_TO_STRING, ERROR_400_HEADERS_LINES_IS_EMPTY, ERROR_500_INTERNAL_SERVER_ERROR, ERROR_400_HEADERS_FAILED_TO_PARSE, ERROR_400_HEADERS_INVALID_HEADER_VALUE, ERROR_400_HEADERS_INVALID_HEADER_NAME};
/// parse the request line into its components
pub fn parse_request_line(
  request_line: String
) -> Result<(Method, Uri, Version), Box<dyn Error>> {
  println!("raw request_line: {:?}", request_line); //todo: remove dev print
  
  let parts:Vec<&str> = request_line.trim().split_whitespace().collect();
  if parts.len() != 3 {
    return Err(ERROR_400_HEADERS_INVALID_REQUEST_LINE.into());
  }

  let (method, uri, version) = (parts[0], parts[1], parts[2]);

  let method = match Method::from_str(method){
    Ok(v) => v,
    Err(e) =>{
      eprintln!("Invalid method: {}\n {}", method, e);
      return Err(ERROR_400_HEADERS_INVALID_METHOD.into())
    },
  };
  
  let uri = match Uri::from_str(uri){
    Ok(v) => v,
    Err(e) =>{
      eprintln!("Invalid uri: {}\n {}", uri, e);
      return Err(ERROR_400_HEADERS_INVALID_METHOD.into())
    }
  };
  
  if version.to_ascii_uppercase() != "HTTP/1.1" {
    eprintln!("Invalid version: {} . According to task requirements it must be HTTP/1.1 \"It is compatible with HTTP/1.1 protocol.\" ", version);
    return Err(ERROR_400_HEADERS_INVALID_VERSION.into());
  }

  println!("PARSED method: {:?}, uri: {:?}, version: {:?}", method, uri, version); //todo: remove dev print

  Ok((method, uri, Version::HTTP_11))

}
