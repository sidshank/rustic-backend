// CORS Solution below comes from: https://github.com/SergioBenitez/Rocket/issues/25
extern crate rocket;

use std::io::Cursor;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Request, Response};
use rocket::http::{Header, ContentType, Method};
pub struct CORS();

impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to requests",
            kind: Kind::Response
        }
    }

    fn on_response(&self, request: &Request, response: &mut Response) {
        if request.method() == Method::Options || 
           response.content_type() == Some(ContentType::JSON) || 
           response.content_type() == Some(ContentType::Plain) {
               
            response.set_header(Header::new("Access-Control-Allow-Origin", "http://localhost:3000"));
            response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, OPTIONS"));
            response.set_header(Header::new("Access-Control-Allow-Headers", "Content-Type"));
            response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
        }

        if request.method() == Method::Options {
            response.set_header(ContentType::Plain);
            response.set_sized_body(Cursor::new(""));
        }
    }
}