#![feature(proc_macro_hygiene, decl_macro)]
extern crate dotenv;
extern crate multipart;

#[macro_use]
extern crate rocket;
extern crate rocket_contrib;

extern crate serde;

use std::io::{BufRead, Cursor};
use std::str::from_utf8;
use std::cell::RefCell;

use dotenv::dotenv;

use multipart::server::Multipart;

use rocket_contrib::json::Json;
use rocket::Data;
use rocket::http::{ContentType, RawStr, Status};
use rocket::response::status::Custom;

use serde::Serialize;

mod s3_interface;
mod cors_hack;

#[derive(Serialize)]
struct S3Object {
    file_name: String,
    presigned_url: String,
    tags: String,
    e_tag: String, // AWS generated MD5 checksum hash for object
}

#[derive(Serialize)]
struct BucketContents {
    data: Vec<S3Object>,
}

#[get("/contents?<filter>")]
fn get_bucket_contents(filter: Option<&RawStr>) -> Result<Json<BucketContents>, Custom<String>> {
    
    let s3_file_manager = s3_interface::S3FileManager::new(None, None, None, None);
    let bucket_contents_maybe = s3_file_manager.get_bucket_contents();
    
    if let None = bucket_contents_maybe {
        return Ok(Json(BucketContents {
            data: Vec::new(),
        }));
    }

    let bucket_list = bucket_contents_maybe.unwrap();

    let mut objects_in_bucket:Vec<S3Object> = Vec::new();
    for bucket_obj in bucket_list {
        
        // Skip folder names, we only care about files
        if bucket_obj.size.unwrap_or(0) == 0 {continue}
        
        if let None = bucket_obj.key {
            return Err(Custom(
                Status::InternalServerError,
                "Encountered bucket objects with no name".into()
            ))
        }

        let file_name = bucket_obj.key.unwrap();
        let tag_req_output = s3_file_manager.get_tags_on_file(file_name.clone());

        let tags_with_categories: Vec<rusoto_s3::Tag> = tag_req_output.into_iter()
                                                                      .filter(|tag| tag.key == "tags")
                                                                      .collect();
        if tags_with_categories.len() > 1 {
            return Err(Custom(
                Status::InternalServerError,
                "Encountered a file with a more than one tag named 'tags'".into()
            ))
        }

        // let tag_value = tags_with_categories[0].value.clone();
        let tag_value = if tags_with_categories.len() == 0 {
            "".to_string()
        } else {
            tags_with_categories[0].value.clone()
        };

        let search_string = filter.unwrap_or_else(|| "".into()).as_str();
        if search_string.len() != 0 {
            if !tag_value.contains(search_string) && !file_name.contains(search_string) {
                // We have a non-empty search string, and it wasn't contained in the
                // string of categories AND the filename. Skip this object and move on
                // to the next one.
                continue;
            }
        }

        let presigned_url = s3_file_manager.get_presigned_url_for_file(file_name.clone());

        let single_object = S3Object {
            file_name: file_name.clone(),
            presigned_url: presigned_url.to_owned(),
            tags: tag_value.to_owned(),
            e_tag: bucket_obj.e_tag.unwrap()
        };
        objects_in_bucket.push(single_object);
    }

    return Ok(Json(BucketContents {
        data: objects_in_bucket,
    }));
}

#[post("/upload", data = "<data>")]
// signature requires the request to have a `Content-Type`
fn upload_file(cont_type: &ContentType, data: Data) -> Result<Custom<String>, Custom<String>> {
    // this and the next check can be implemented as a request guard but it seems like just
    // more boilerplate than necessary
    if !cont_type.is_form_data() {
        return Err(Custom(
            Status::BadRequest,
            "Content-Type not multipart/form-data".into()
        ));
    }

    let (_, boundary) = cont_type.params().find(|&(k, _)| k == "boundary").ok_or_else(
        | | Custom(
            Status::BadRequest,
            "`Content-Type: multipart/form-data` boundary param not provided".into()
        )
    )?;

    let mut d = Vec::new();
    data.stream_to(&mut d).expect("Unable to read");

    // The hot mess that ensues is some weird combination of the two links that follow
    // and a LOT of hackery to move data between closures.
    // https://github.com/SergioBenitez/Rocket/issues/106
    // https://github.com/abonander/multipart/blob/master/examples/rocket.rs
    let mut mp = Multipart::with_body(Cursor::new(d), boundary);
    let file_name_outer = RefCell::new(String::new());
    let tags_outer = RefCell::new(String::new());
    let data_outer = RefCell::new(Vec::<u8>::new());

    mp.foreach_entry(|mut entry| {
        if *entry.headers.name == *"fileName" { 
            let file_name_vec = entry.data.fill_buf().unwrap().to_owned();
            let file_name_inner = from_utf8(&file_name_vec).unwrap();
            *file_name_outer.borrow_mut() = file_name_inner.to_string();
        } else if *entry.headers.name == *"tags" {
            let tags_vec = entry.data.fill_buf().unwrap().to_owned();
            let tags = from_utf8(&tags_vec).unwrap();
            *tags_outer.borrow_mut() = tags.to_string();
        } else if *entry.headers.name == *"file" {
            let file_data_vec = entry.data.fill_buf().unwrap().to_owned();
            *data_outer.borrow_mut() = file_data_vec;
        }
    }).expect("Unable to iterate");

    let file_name = file_name_outer.into_inner();
    let s3_file_manager = s3_interface::S3FileManager::new(None, None, None, None);
    
    s3_file_manager.put_file_in_bucket(file_name.clone(), data_outer.into_inner());

    let concatenated_tag_string = tags_outer.into_inner();
    let tag_name_val_pairs = vec![("tags".to_string(), concatenated_tag_string)];
    s3_file_manager.put_tags_on_file(file_name, tag_name_val_pairs);

    return Ok(
        Custom(Status::Ok, "Image Uploaded".to_string())
    );
}

fn main() {
    dotenv().expect(".env file not found");
    rocket::ignite().attach(
        cors_hack::CORS()
    ).mount(
        "/", 
        routes![get_bucket_contents, upload_file]
    ).launch();
}