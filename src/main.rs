extern crate ftp;
use ftp::{ FtpStream };

#[macro_use]
extern crate clap;

use dotenv;

use std::path::Path;
use std::io::{
    BufReader,
    Result as IoResult
};
use std::fs::{
    File,
    read_dir,
};
use std::env;



fn main() -> Result<(), String> {

    dotenv::dotenv().unwrap();

    let matches = clap_app!(myapp =>
        (version: "1.0")
        (author: "00benallen")
        (about: "Simple FTP deployment program")
        (@arg SRC_DIR: --src +takes_value "Sets the source directory for deploy")
        (@arg DST_DIR: --dst +takes_value "Sets the destination directory for deploy")
        (@arg DEPLOY_ASSETS: -a "If present, will also upload the /assets in the source folder")
    ).get_matches();

    // Gets a value for config if supplied by user, or defaults to "default.conf"
    let src_dir = matches.value_of("SRC_DIR").unwrap_or("./dist");

    // Gets a value for config if supplied by user, or defaults to "default.conf"
    let upload_assets = matches.is_present("DEPLOY_ASSETS");

    // get environment variables
    let host = env::var("FTP_HOST").map_err(|_| "FTP_HOST env variable not set").unwrap();
    let username = env::var("FTP_USERNAME").map_err(|_| "FTP_USERNAME env variable not set").unwrap();
    let password = env::var("FTP_PASSWORD").map_err(|_| "FTP_PASSWORD env variable not set").unwrap();

    // Create a connection to an FTP server and authenticate to it.

    let mut ftp_stream = FtpStream::connect(host).map_err(|_| "Could not connect to host")?;
    ftp_stream.login(&username, &password).map_err(|_| "Login failed")?;
    
    // Change into a new directory, relative to the one we are currently in.
    ftp_stream.cwd("public_html").map_err(|_| "Could not navigate to public_html")?;

    let files = ftp_stream.nlst(None).map_err(|_| "Could not list files on server for deletion")?;

    let patterns_to_delete = vec![
        "main-", 
        "3rdpartylicenses.txt", 
        "favicon.ico",
        "index.html",
        "polyfills-",
        "runtime-",
        "styles"];

    if upload_assets {
        ftp_stream.rmdir("/assets").map_err(|_| "Could not delete assets folder")?;
        println!("Assets folder deleted");
    }

    for file_name in files {
        for pattern in &patterns_to_delete {
            if file_name.contains(pattern) {
                ftp_stream.rm(&file_name).map_err(|_| "Could not delete file")?;
                println!("File {} deleted", file_name);
            }
        }
    }

    upload_recursive(Path::new(src_dir), &mut ftp_stream, upload_assets).map_err(|_| "Upload failed")?;
    println!("Files transferred successfully!");

    // Terminate the connection to the server.
    ftp_stream.quit().map_err(|_| "Connection could not be closed")?;

    Ok(())
}

fn upload_recursive(dir: &Path, ftp_stream: &mut FtpStream, upload_assets: bool) -> IoResult<()> {

    if dir.is_dir() {
        for entry in read_dir(dir)? {

            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {

                if  path.to_string_lossy().contains("assets") {
                    if  upload_assets {
                        upload_directory(&path, ftp_stream, upload_assets)?;
                    } else {
                        println!("Assets folder found at {:?}, skipping", path);
                    }
                } else {
                    upload_directory(&path, ftp_stream, upload_assets)?;
                }

            } else if &path.to_string_lossy() != "./dist/.DS_Store" {
                let mut reader = BufReader::new(File::open(&path)?);
                let pwd = ftp_stream.pwd().unwrap();
                println!("Uploading file from source {:?} to destination {:?}", path, pwd);
                let new_file_name = path.file_name().unwrap().to_string_lossy();
                ftp_stream.put(&new_file_name, &mut reader).unwrap();
            }

        }
    }

    ftp_stream.cdup().unwrap();
    println!("Returning up to parent {:?}", ftp_stream.pwd().unwrap());
    Ok(())
}

fn upload_directory(path: &Path, ftp_stream: &mut FtpStream, upload_assets: bool) -> IoResult<()> {
    let new_dir_name = path.file_name().unwrap().to_string_lossy();
    ftp_stream.mkdir(&new_dir_name).unwrap();
    ftp_stream.cwd(&new_dir_name).unwrap();
    println!("Created directory on server {:?}", new_dir_name);
    println!("Recursing to directory {:?}", path);
    upload_recursive(&path, ftp_stream, upload_assets)?;

    Ok(())
}
