use std::path::PathBuf;

use http::{Response, Request, StatusCode};

use crate::files::check::is_implemented_error_page;
use crate::handlers::response_500::custom_response_500;
use crate::server::core::ServerConfig;
use crate::handlers::response_::{response_default_static_file, force_status};
use crate::handlers::response_4xx::custom_response_4xx;


/// handle all requests, except cgi, and except uploads.
/// 
/// Also, in case of uri is directory, the task requires to return default file,
/// according to server config. So in this case, there is no need to check the method,
/// allowed for route.
pub fn handle_all(
  request: &Request<Vec<u8>>,
  cookie_value:String,
  zero_path_buf: PathBuf,
  server_config: ServerConfig,
) -> Response<Vec<u8>>{
  // todo: refactor path check to os separator instead of hardcoding of / ... probably
  
  // replace /uploads/ to /, to prevent wrong path. The uploads files served separately on the upper level
  let binding_path_string = request.uri().path().replacen("uploads/", "", 1);
  let mut path_str = binding_path_string.as_str();
  
  // cut first slash
  if path_str.starts_with("/"){ path_str = &path_str[1..]; }
  println!("path {}", path_str); // todo: remove dev prints
  
  // check if path is error page
  let is_error_page = is_implemented_error_page(path_str);
  // path to site folder in static folder
  let relative_static_path_string =
  if is_error_page {
    println!("is_implemented_error_page"); //todo: remove dev print
    let file_name = match path_str.split('/').last(){
      Some(v) => v,
      None => {
        eprintln!("ERROR: path_str.split('/').last()\nFailed with path {}", path_str);
        eprintln!(" Must never fire, because path checked/confirmed before.\nSo return [500]");
        return custom_response_500(
          request,
          cookie_value,
          zero_path_buf,
          server_config,
        );
      }
    };
    format!("static/{}/{}", server_config.error_pages_prefix, file_name)
  }
  else { format!("static/{}/{}", server_config.static_files_prefix, path_str)};
  
  println!("relative_static_path_string {}", relative_static_path_string);
  
  let absolute_path_buf = zero_path_buf.join(relative_static_path_string);
  println!("absolute_path_buf {:?}", absolute_path_buf);
  
  // check if path is directory, then return default file as task requires
  if path_str.ends_with("/") || absolute_path_buf.is_dir() {
    return response_default_static_file(
      request,
      cookie_value,
      zero_path_buf,
      server_config,
    );
  } else if !absolute_path_buf.is_file() {
    
    eprintln!("ERROR:\n------------\nIS NOT A FILE\n-------------");
    
    return custom_response_4xx(
      request, 
      cookie_value,
      zero_path_buf, 
      server_config,
      StatusCode::NOT_FOUND,
    )
  } // check if file exists or return 404
  
  
  let parts: Vec<&str> = path_str.split('/').collect();
  println!("=== parts {:?}", parts); // todo: remove dev prints
  
  // check if path is inside routes, then get methods allowed for this path
  // create empty vector if path is not inside routes
  
  // rust is crap , you can not just return vec from match, nested more then 1 level
  
  // this is second approach, limited , so not used at the moment. First one was fixed
  // using binding ... magic. facepalm

  // let mut allowed_methods: Vec<String> = Vec::new(); 
  // if is_error_page {
  //   allowed_methods.push("GET".to_string())
  // } else {
  //   let addition = match server_config.routes.get(path_str){
  //     Some(v) => {v},
  //     None => {
  //       eprintln!("ERROR: path {} is not inside routes", path_str);
  //       return custom_response_4xx(
  //         request,
  //         zero_path_buf,
  //         server_config,
  //         http::StatusCode::NOT_FOUND,
  //       )
  //     }
  //   };
  //   allowed_methods.append(&mut addition.to_vec()) // and even this looks ugly
  // }
  
  // first fixed approach
  let mut rust_handicap_binding:Vec<String> = Vec::new();
  let allowed_methods: &Vec<String> = match server_config.routes.get(path_str){
    Some(v) => {v},
    None => {
      if is_error_page {
        rust_handicap_binding.push("GET".to_string());
        &rust_handicap_binding
        
      } else {
        eprintln!("ERROR: path {} is not inside routes", path_str);
        return custom_response_4xx(
          request,
          cookie_value,
          zero_path_buf,
          server_config,
          http::StatusCode::NOT_FOUND,
        )
      }
    }
  };
  

  // check if method is allowed for this path or return 405
  let request_method_string = request.method().to_string();
  if !allowed_methods.contains(&request_method_string){
    eprintln!("ERROR: method {} is not allowed for path {}", request_method_string, path_str);
    return custom_response_4xx(
      request,
      cookie_value,
      zero_path_buf,
      server_config,
      http::StatusCode::METHOD_NOT_ALLOWED,
    )
  }
  
  // read the file. if error, then return error 500 response
  let file_content = match std::fs::read(absolute_path_buf.clone()){
    Ok(v) => v,
    Err(e) => {
      eprintln!("ERROR: Failed to read file: {}", e);
      return custom_response_500(
        request,
        cookie_value,
        zero_path_buf,
        server_config
      )
    }
  };
  
  let mut response = match Response::builder()
  .status(
    force_status(
      zero_path_buf.clone(),
      absolute_path_buf.clone(),
      server_config.clone(),
    )
  )
  .header("Set-Cookie", cookie_value.clone())
  .body(file_content)
  {
    Ok(v) => v,
    Err(e) => {
      eprintln!("ERROR: Failed to create response with file: {}", e);
      return custom_response_500(
        request,
        cookie_value.clone(),
        zero_path_buf,
        server_config)
      }
    };
    
    // get file mime type using mime_guess, or use the text/plain
    let mime_type = match mime_guess::from_path(absolute_path_buf.clone()).first(){
      Some(v) => v.to_string(),
      None => "text/plain".to_string(),
    };
    // println!("\n-------\n\nmime_type {}\n\n----------\n", mime_type); //todo: remove dev print
    
    response.headers_mut().insert(
      "Content-Type",
      match mime_type.parse(){
        Ok(v) => v,
        Err(e) => {
          eprintln!("ERROR: Failed to parse mime type: {}", e);
          "text/plain".parse().unwrap()
        }
      }
    );
    
    response
    
  }
  