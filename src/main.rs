#![feature(proc_macro_hygiene, decl_macro)]
extern crate dotenv;
extern crate multipart;
#[macro_use]
extern crate rocket;
extern crate rocket_contrib;
extern crate rocket_cors;
extern crate serde;

use std::io::{BufRead, Cursor};
use std::str::from_utf8;

use dotenv::dotenv;
use multipart::server::Multipart;
use rocket_contrib::json::Json;
use rocket_cors::{AllowedHeaders, AllowedOrigins, Error};
use rocket::Data;
use rocket::http::{ContentType, Method, RawStr, Status};
use rocket::response::status::Custom;
use rusoto_s3::Tag;

mod s3_interface;

use s3_interface::{BucketContents, S3Object};

#[derive(Debug)]
enum S3ObjectError {
    FileWithNoName,
    MultipleTagsWithSameName,
}

#[get("/contents?<filter>")]
fn get_bucket_contents(filter: Option<&RawStr>) -> Result<Json<BucketContents>, Custom<String>> {
    
    let s3_file_manager = s3_interface::S3FileManager::new(None, None, None, None);
    let bucket_contents_maybe = s3_file_manager.get_bucket_contents();
    if let None = bucket_contents_maybe {
        return Ok(Json(BucketContents::empty_bucket()));
    }
    let bucket_list = bucket_contents_maybe.unwrap();
    let search_string = filter.unwrap_or_else(|| "".into()).as_str();
    let should_search = search_string.len() > 0;

    let bucket_contents: Result<Vec<S3Object>, S3ObjectError> = bucket_list
        .into_iter()
        .filter(|bucket_obj| bucket_obj.size.unwrap_or(0) != 0) // Eliminate folders
        .map(|bucket_obj| {
            if let None = bucket_obj.key {
                return Err(S3ObjectError::FileWithNoName);
            }

            let file_name = bucket_obj.key.unwrap();
            let e_tag = bucket_obj.e_tag.unwrap_or(String::new());
            let tag_req_output = s3_file_manager.get_tags_on_file(file_name.clone());
            let tags_with_categories: Vec<Tag> = tag_req_output.into_iter()
                                                            .filter(|tag| tag.key == "tags")
                                                            .collect();
            if tags_with_categories.len() > 1 {
                return Err(S3ObjectError::MultipleTagsWithSameName);
            }

            let tag_value = if tags_with_categories.len() == 0 {
                "".to_string()
            } else {
                tags_with_categories[0].value.clone()
            };

            if should_search &&
                !tag_value.contains(search_string) &&
                !file_name.contains(search_string) {
                Ok(S3Object::new(
                    file_name,
                    e_tag,
                    tag_value,
                    String::new(),
                    true,
                ))
            } else {
                // TODO: We don't have to go to S3 everytime we need a pre-signed link
                // Pre-signed links should probably be stored in a cache with a reasonable
                // expiry and fetched from the cache whenever needed.
                let presigned_url = s3_file_manager.get_presigned_url_for_file(
                    file_name.clone()
                );
                Ok(S3Object::new(
                    file_name,
                    e_tag,
                    tag_value,
                    presigned_url,
                    false,
                ))
            }
        })
        .collect();

        match bucket_contents {
            Err(why) => match why {
                S3ObjectError::FileWithNoName => Err(Custom(
                    Status::InternalServerError,
                    "Encountered bucket objects with no name".into()
                )),
                S3ObjectError::MultipleTagsWithSameName => Err(Custom(
                    Status::InternalServerError,
                    "Encountered a file with a more than one tag named 'tags'".into()
                ))
            },
            Ok(s3_objects) => {
                let visible_s3_objects: Vec<S3Object> = s3_objects.into_iter()
                                                                  .filter(|obj| !obj.is_hidden())
                                                                  .collect();
                Ok(Json(BucketContents::new(visible_s3_objects)))
            }
        }
}

#[post("/upload", data = "<data>")]
// signature requires the request to have a `Content-Type`. The preferred way to handle the incoming
// data would have been to use the FromForm trait as described here: https://rocket.rs/v0.4/guide/requests/#forms
// Unfortunately, file uploads are not supported through that mechanism since a file upload is performed as a
// multipart upload, and Rocket does not currently (As of v0.4) support this. 
// https://github.com/SergioBenitez/Rocket/issues/106
fn upload_file(cont_type: &ContentType, data: Data) -> Result<Custom<String>, Custom<String>> {
    // this and the next check can be implemented as a request guard but it seems like just
    // more boilerplate than necessary
    if !cont_type.is_form_data() {
        return Err(Custom(
            Status::BadRequest,
            "Content-Type not multipart/form-data".into()
        ));
    }

    let (_, boundary) = cont_type.params()
                                 .find(|&(k, _)| k == "boundary")
                                 .ok_or_else(
        || Custom(
            Status::BadRequest,
            "`Content-Type: multipart/form-data` boundary param not provided".into()
        )
    )?;

    // The hot mess that ensues is some weird combination of the two links that follow
    // and a LOT of hackery to move data between closures.
    // https://github.com/SergioBenitez/Rocket/issues/106
    // https://github.com/abonander/multipart/blob/master/examples/rocket.rs
    let mut d = Vec::new();
    data.stream_to(&mut d).expect("Unable to read");
    let mut mp = Multipart::with_body(Cursor::new(d), boundary);

    let mut file_name = String::new();
    let mut categories_string = String::new();
    let mut raw_file_data = Vec::new();

    mp.foreach_entry(|mut entry| {
        if *entry.headers.name == *"fileName" { 
            let file_name_vec = entry.data.fill_buf().unwrap().to_owned();
            file_name = from_utf8(&file_name_vec).unwrap().to_string()
        } else if *entry.headers.name == *"tags" {
            let tags_vec = entry.data.fill_buf().unwrap().to_owned();
            categories_string = from_utf8(&tags_vec).unwrap().to_string();
        } else if *entry.headers.name == *"file" {
            raw_file_data = entry.data.fill_buf().unwrap().to_owned()
        }
    }).expect("Unable to iterate");

    let s3_file_manager = s3_interface::S3FileManager::new(None, None, None, None);
    s3_file_manager.put_file_in_bucket(file_name.clone(), raw_file_data);

    let tag_name_val_pairs = vec![("tags".to_string(), categories_string)];
    s3_file_manager.put_tags_on_file(file_name, tag_name_val_pairs);

    return Ok(
        Custom(Status::Ok, "Image Uploaded".to_string())
    );
}

fn main() -> Result<(), Error> {
    dotenv().expect(".env file not found");

    let allowed_origins = AllowedOrigins::some_exact(&["http://localhost:3000"]);

    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get, Method::Post].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::some(&["Content-Type", "Authorization", "Accept"]),
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()?;


    rocket::ignite().attach(cors)
                    .mount("/", routes![get_bucket_contents, upload_file])
                    .launch();

    Ok(())
}
