extern crate notify;
extern crate fs_extra;

use ctrlc;
use clap::{Arg, App};
use notify::{Watcher, RecursiveMode, watcher};
use std::time::Duration;
use std::sync::mpsc::channel;
use std::path::{PathBuf};


fn main() {
    let matches = App::new("Syncfolder")
        .version("1.0.0")
        .author("Markus Flür <markusfluer@markusfluer.de>")
        .about("Synchronize a folder with another and watch for changes.")
        .arg(Arg::with_name("input")
                 .short("i")
                 .long("input")
                 .required(true)
                 .takes_value(true)
                 .help("Your input folder"))
        .arg(Arg::with_name("output")
                 .short("o")
                 .long("output")
                 .required(true)
                 .takes_value(true)
                 .help("The folder you want to be synchronized"))
        .arg(Arg::with_name("noinitial")
                 .short("n")
                 .long("noinitial")
                 .help("Use if it should not do a full sync at the beginning"))
        .get_matches();

    println!("... Setting up");
       
    let first_arg =  matches.value_of("input").unwrap_or("");
    let second_arg = matches.value_of("output").unwrap_or("");
    let third_arg = matches.is_present("noinitial");
    
    if first_arg == "" || second_arg == "" {
        println!("No input and output argument set. Exiting. For help use `syncfolders --help´");
        return;
    }

    if !third_arg {
        println!("... Synchronizing folder before starting watch");
        let mut options = fs_extra::dir::CopyOptions::new(); 
        options.overwrite = true;
        options.copy_inside = true;
        options.content_only = true;
        match fs_extra::dir::copy(&first_arg, &second_arg, &options) {
            Ok(_) => {
                println!("... Folders successfully synchronized.");
            },
            Err(e) => {
                println!("... Error synchronizing folders: {:?}", e);
            }
        }
    } else {
        println!("... Start watch without synchronizing");
    }

    // watcher channel
    let (tx, rx) = channel();
    let (txw, rxw) = channel();
    let txx = txw.clone();

    println!("... Starting sync process");

    let opt_in_path = String::from(first_arg);
    let opt_out_path = String::from(second_arg);
    
    
    // Create a watcher object, delivering debounced events.
    // The notification back-end is selected based on the platform.
    let mut watcher = watcher(txw, Duration::from_secs(3)).unwrap();
    

    ctrlc::set_handler(move || {
        let _1 = tx.send(1).unwrap();
        let _2 = txx.send(
            notify::DebouncedEvent::Error(
                notify::Error::Generic(String::from("Stopping")), 
                Option::from(PathBuf::from("/")))
            ).unwrap();
    }).unwrap();

    watcher.watch(&opt_in_path, RecursiveMode::Recursive).unwrap();
    println!("... Watcher started, you can start now!");

    loop {
        
        if rx.try_recv().unwrap_or(0) == 1 { 
            println!("... Closing"); 
            println!("Thank you for using syncfolders!"); 
            break;
        }

        match rxw.recv() {
            Ok(event) => { on_watch_event(event, &opt_in_path, &opt_out_path); }
            Err(e) => { println!("Error in watcher: {:?}", e); break;}
        }

    }

}

fn get_relative_file_reference(in_file_ref: &PathBuf, in_path: &PathBuf ) -> PathBuf {
    let in_file = in_file_ref.clone();

    match in_file.strip_prefix(&in_path) {
        Ok(path_returned) => { return path_returned.to_path_buf(); },
        Err(e) => {
            println!("Error in stripping path prefix:"); 
            println!("in_file: {:?}", in_file);
            println!("to be stripped part: {:?}", &in_path);
            println!("Error: {:?}", e);
            return in_file.to_path_buf();
        }
    }
}

fn get_relative_file_reference_for_remove(in_file_ref: &PathBuf, in_path: &PathBuf ) -> PathBuf {
    let in_file = in_file_ref.clone();

    match in_file.strip_prefix(&in_path) {
        Ok(path_returned) => { return path_returned.to_path_buf(); },
        Err(e) => {
            println!("Error in stripping path prefix:"); 
            println!("in_file: {:?}", in_file);
            println!("to be stripped part: {:?}", &in_path);
            println!("Error: {:?}", e);
            return in_file.to_path_buf();
        }
    }
}

fn map_to_complete_file(file_ref: &PathBuf, out_path: &PathBuf ) -> PathBuf  {
    let current_dir = std::env::current_dir().unwrap();
    return current_dir.join(&out_path).join(file_ref);
}


fn copy_to_output(in_file: &PathBuf, in_path: &str, out_path: &str ) {    
    let current_dir = std::env::current_dir().unwrap();

    let file_ref_buf = in_file.canonicalize().unwrap();
    let in_path_buf = current_dir.join(PathBuf::from(in_path)).canonicalize().unwrap();
    let out_path_buf = current_dir.join(PathBuf::from(out_path)).canonicalize().unwrap();
    
    let file_ref = get_relative_file_reference(&file_ref_buf, &in_path_buf);
    let write_out_file  = map_to_complete_file(&file_ref, &out_path_buf);
    let write_out_path  = write_out_file.parent();
    
    if in_file.is_file() {
        let mut options = fs_extra::file::CopyOptions::new(); 
        options.overwrite = true;

        match fs_extra::dir::create_all(write_out_path.unwrap(), false) {
            Ok(_) => {},
            Err(_) => {
                println!("Error copying file. Could not created path"); 
            }
        }
        
        match fs_extra::file::copy(&in_file, &write_out_file, &options)  {
            Ok(_) => { println!("File {:?} updated successfully", file_ref); },
            Err(e) => { 
                println!("Error copying file:"); 
                println!("Input: {:?}", in_file); 
                println!("Relateive path: {:?}", file_ref); 
                println!("Output: {:?}", write_out_file); 
                println!("Error: {:?}", e); 
                touch(&in_file);
            }
        };
    } else {
        if in_file.is_dir() && !write_out_file.exists() {
            match fs_extra::dir::create(&write_out_file, false) {
                Ok(_) => { println!("Folder {:?} updated successfully", file_ref); },
                Err(e) => { 
                    println!("Error creating folder:"); 
                    println!("Input: {:?}", in_file); 
                    println!("Relateive path: {:?}", file_ref); 
                    println!("Output: {:?}", write_out_path); 
                    println!("Error: {:?}", e); 
                }
            };
        }
    }
}

fn remove_in_target(in_file: &PathBuf, in_path: &str, out_path: &str ) {    
    let current_dir = std::env::current_dir().unwrap();

    let out_path_buf = current_dir.join(PathBuf::from(out_path));
    let in_path_buf = current_dir.join(PathBuf::from(in_path));
    
    let file_ref = get_relative_file_reference_for_remove(&in_file, &in_path_buf);
    let write_out_path  = map_to_complete_file(&file_ref, &out_path_buf);
    
    if write_out_path.is_file() {
        match fs_extra::file::remove(&write_out_path) {
            Ok(_) => { println!("File {:?} deleted in target", file_ref); },
            Err(e) => { 
                println!("Error removing file:"); 
                println!("Input: {:?}", in_file); 
                println!("Relative path: {:?}", file_ref); 
                println!("Output: {:?}", write_out_path); 
                println!("Error: {:?}", e); 
            }
        };
    } else {
        match fs_extra::dir::remove(&write_out_path) {
            Ok(_) => { println!("Directory {:?} deleted in target", file_ref); },
            Err(e) => { 
                println!("Error removing directory:"); 
                println!("Input: {:?}", in_file); 
                println!("Relative path: {:?}", file_ref); 
                println!("Output: {:?}", write_out_path); 
                println!("Error: {:?}", e); 
            }
        };

    }
}

fn on_watch_event(evt: notify::DebouncedEvent, opt_in_path: &std::string::String, opt_out_path: &std::string::String) {
    let in_path = opt_in_path.clone();
    let out_path = opt_out_path.clone();

    match evt {
        notify::DebouncedEvent::NoticeWrite(_path) => {},
        notify::DebouncedEvent::NoticeRemove(_path) => {},
        notify::DebouncedEvent::Create(path) => {
            copy_to_output(&path,&in_path,&out_path);
            
        },
        notify::DebouncedEvent::Write(path) => {
            copy_to_output(&path,&in_path,&out_path);
        },
        notify::DebouncedEvent::Chmod(_path) => {},
        notify::DebouncedEvent::Remove(path) => {
            remove_in_target(&path,&in_path,&out_path);
        },
        notify::DebouncedEvent::Rename(oldpath, newpath) => {
            remove_in_target(&oldpath,&in_path,&out_path);
            copy_to_output(&newpath,&in_path,&out_path);
        },
        notify::DebouncedEvent::Rescan => {}
        notify::DebouncedEvent::Error(err, _path) => {
            match err {
                notify::Error::Generic(errtype) => {
                    if errtype == String::from("Stopping") {
                        return;
                    }
                },
                _ => {}
            }
        }
    }
}


fn touch(file_path: &std::path::PathBuf) -> bool {
    let f = std::fs::File::open(file_path);

    if f.is_err() {
        return false;
    }

    match f.unwrap().sync_all() {
        Ok(_) => {return true;},
        Err(_) => {return false;},
    }
}