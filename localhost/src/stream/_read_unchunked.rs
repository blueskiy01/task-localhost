use std::error::Error;
use std::time::{Instant, Duration};

use async_std::io;
use async_std::net::TcpStream;
use futures::AsyncReadExt;

use crate::debug::append_to_file;
use crate::stream::errors::{ERROR_400_HEADERS_READ_TIMEOUT, ERROR_400_HEADERS_READING_STREAM, ERROR_400_BODY_SUM_CHUNK_SIZE_READ_TIMEOUT, ERROR_400_BODY_SUM_CHUNK_SIZE_READING_STREAM, ERROR_400_BODY_SUM_CHUNK_SIZE_PARSE, ERROR_400_BODY_CHUNKED_BUT_ZERO_SUM_CHUNK_SIZE, ERROR_400_BODY_CHUNK_SIZE_READ_TIMEOUT, ERROR_400_BODY_CHUNK_SIZE_READING_STREAM, ERROR_400_BODY_CHUNK_SIZE_PARSE, ERROR_400_BODY_CHUNK_READ_TIMEOUT, ERROR_400_BODY_CHUNK_READING_STREAM, ERROR_400_BODY_CHUNK_IS_BIGGER_THAN_CHUNK_SIZE, ERROR_400_HEADERS_FAILED_TO_PARSE, ERROR_400_BODY_BUFFER_LENGHT_IS_BIGGER_THAN_CONTENT_LENGTH, ERROR_400_BODY_READ_TIMEOUT, ERROR_400_DIRTY_BODY_READ_TIMEOUT, ERROR_400_BODY_READING_STREAM, ERROR_413_BODY_SIZE_LIMIT};


pub async fn read_unchunked(
  stream: &mut TcpStream,
  headers_buffer: &mut Vec<u8>,
  body_buffer: &mut Vec<u8>,
  client_body_size: usize,
  timeout: Duration,
  global_error_string: &mut String,
) {
  
  // if request is not chunked
  // not clear way to manage not clear standard.
  // To manage cases when there is unchunked body, but
  // without Content-Length header(because this header is not mandatory),
  // try to implement the second timeout for body read in this case.
  // it will be x5 times shorter than the timeout incoming parameter.
  
  println!("THE REQUEST IS NOT CHUNKED");
  
  // Start the timer for body read
  let start_time = Instant::now();
  let mut body_size = 0;
  
  let mut buf:Vec<u8> = Vec::new();
  let dirty_timeout = timeout / 5;
  let dirty_start_time = Instant::now();
  
  let content_length_header_not_found =
  !String::from_utf8_lossy(&headers_buffer).contains("Content-Length: ")
  || !String::from_utf8_lossy(&headers_buffer).contains("content-length: ");
  
  let content_length = if content_length_header_not_found {
    println!("ERROR: Content-Length header not found in headers_buffer of unchunked body. Continue with MAX-1 content_length of \ndirty body\n.");
    usize::MAX-1
    
  } else {
    // extract content length from headers_buffer and parse it to usize
    let headers_buffer_slice = headers_buffer.as_slice();
    let headers_str = String::from_utf8_lossy(headers_buffer_slice);
    let content_length_header = 
    headers_str.split("\r\n").find(|&s| s.starts_with("Content-Length: "));
    
    let content_length_str = match content_length_header {
      Some(v) => v.trim_start_matches("Content-Length: "),
      None => {
        eprintln!("ERROR: Failed to get Content-Length header from headers_buffer of unchunked body. Continue with 0 content_length of \ndirty body\n.");
        "0" // help to content_length to be
      } 
    };
    
    match content_length_str.parse() {
      Ok(v) => v,
      Err(e) => {
        eprintln!("ERROR: Failed to parse content_length_str: {}\n {}", content_length_str, e);
        *global_error_string = ERROR_400_HEADERS_FAILED_TO_PARSE.to_string();
        return 
      }
    }
  };
  
  println!("content_length: {}", content_length); //todo: remove later
  
  loop{
    // check the body_buffer length
    if content_length > 0{
      if body_buffer.len() == content_length{
        break;
      } else if body_buffer.len() > content_length{
        eprintln!("ERROR: body_buffer.len() > content_length");
        *global_error_string = ERROR_400_BODY_BUFFER_LENGHT_IS_BIGGER_THAN_CONTENT_LENGTH.to_string();
        return 
      }
    }
    
    // Check if the timeout has expired
    if start_time.elapsed() >= timeout {
      eprintln!("ERROR: Body read timed out");
      append_to_file("ERROR: Body read timed out").await;
      *global_error_string = ERROR_400_BODY_READ_TIMEOUT.to_string();
      return 
    } else {
      println!("body_buffer.len(): {}", body_buffer.len()); //todo: remove later
      println!("time {} < timeout {}", start_time.elapsed().as_millis(), timeout.as_millis()); //todo: remove later
    }
    
    if content_length_header_not_found // potentilal case of dirty body
    && dirty_start_time.elapsed() >= dirty_timeout
    {
      if body_buffer.len() == 0{
        break; // let is say that there is no body in this case, so continue
      } else {
        eprintln!("ERROR: Dirty body read timed out.\n = File: {}, Line: {}, Column: {}", file!(), line!(), column!()); //todo: remove later
        *global_error_string = ERROR_400_DIRTY_BODY_READ_TIMEOUT.to_string();
        return 
      }
    }
    
    println!(" before \"match stream.read(&mut buf).await {{\"read from the stream one byte at a time"); //todo: remove later FIRES ONCE
    // Read from the stream one byte at a time
    match stream.read_to_end(&mut buf).await {
      
      Ok(n) => {
        println!("read one byte NEVER FIRES"); //FIX: remove later. NEVER FIRES
        body_size += n;
        
        // Check if the body size is bigger than client_body_size
        if body_size > client_body_size {
          eprintln!("ERROR: Body size is bigger than client_body_size limit: {} > {}", body_size, client_body_size);
          *global_error_string = ERROR_413_BODY_SIZE_LIMIT.to_string();
          return 
        }
        
        // Successfully read n bytes from stream
        body_buffer.extend_from_slice(&buf[..n]);
        
        // Check if the end of the stream has been reached
        if n < buf.len() {
          println!("read EOF reached relatively, because buffer not full after read");
          break;
        }
      },
      Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
        eprintln!("ERROR: Stream is not ready yet, try again later");
        async_std::task::yield_now().await;
        // Stream is not ready yet, try again later
        continue;
      },
      Err(e) => {
        // Other error occurred
        eprintln!("ERROR: Reading from stream: {}", e);
        *global_error_string = ERROR_400_BODY_READING_STREAM.to_string();
        return 
      },
    }
    
    println!(" AFTER \"match stream.read(&mut buf).await {{\"read from the stream one byte at a time"); //fix: remove later NEVER FIRES
    
  }
  
  
  
  
  // todo!("read_unchunked")
  
  
}