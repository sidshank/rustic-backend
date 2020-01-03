# RustIC Backend

A Rust based service to serve files from an S3 backend.

## Why does this project exist?

It was born at the 2019 year-end Prodigy Education hackathon, and was written with almost no previous knowledge of Rust, and just a cursory knowledge of S3 capabilities.

My goals were to:

- [x] Get my feet wet with Rust
- [x] Explore what is took to build a web-app using Rust (I used [rocket](https://github.com/SergioBenitez/Rocket)).
- [x] Explore S3 capabilities for a work-related project.

More specifically, I wanted to explore:

- [x] S3 Object browsing / puts / gets via a Rust S3 SDK (I chose [rusoto](https://github.com/rusoto/rusoto), although other SDKs do exist).
- [x] The ability to add metadata and tags to S3 objects.
- [x] Generating presigned links for S3 Objects.
- [ ] S3 Roles and Permissions

## Prerequisites

0. An AWS account, with an S3 bucket already setup.
1. Create a .env file in the root folder with the following environment variables and their values. For example:
    ```
    RUSTIC_IMAGES_BUCKET_NAME=MyBucketName
    RUSTIC_IMAGES_ACCESS_KEY=keywithalotofcharacters
    RUSTIC_IMAGES_SECRET_KEY=aninsanelylongsecretkeythatsuncrackable
    RUSTIC_IMAGES_AWS_REGION=us-east-1
    ```
    The app can easily be confused to your the aws configuration settings on your machine, I just didn't do it. 🤷‍♂️ 
1. You will need to install [Rust](https://www.rust-lang.org/tools/install) including rustup.
2. Switch to the nightly version of Rust, since rocket 🚀 depends on the nightly version of Rust for now (As of Jan 2020). Personally, I would recommmend switching to the nightly version only for this project, by executing 

    `rustup override set nightly` 

    in the project folder

## Available Scripts

In the Project directory, you can run:

`cargo build` -> Pulls in all dependencies specified in Cargo.toml, and then compiles / builds the project.
`cargo run` -> Should start the web app on [http://localhost:8000](http://localhost:8000]

## App Frontend

The App frontend repo can be accessed [here](https://github.com/sidshank/rustic-frontend). The README in that repo should allow you to be up and running.

## Remember

1. In case the code didn't already give it away, this app is not production ready -> Hasn't been tested with anything resembling a reasonable service load and is probably easy to DDoS by throwing a massive file at it.
2. The code isn't idiomatic Rust, not even close. There're hacky shortcuts, plenty of borrowing and cloning of variables, and only a minimal arrangement of code into modules. There are also nasty / inefficient API use patterns, such as generating a pre-signed link for every object in a vector, in a for loop monkey 🐒🤦‍♂️ , but I was focusing on delivering **functionality**, *quickly*, sacrificing correctness and efficiency ... And, I learnt a ton about Rust, Rocket and Rusoto while doing it, which is what really counts.
3. The focus of my project was on using S3 as an object store for image files, which may explain some of the env var names, but the backend should work for files of any type.
