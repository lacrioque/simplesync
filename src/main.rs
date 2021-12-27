extern crate notify;

use notify::{Watcher, RecursiveMode, watcher};
use std::time::Duration;
use std::sync::mpsc::channel;
use ctrlc;
use std::env;
use std::path::{PathBuf};
use std::fs::{copy, remove_file};


fn main() {
    println!("Syncing folders, Events will be mapped if Log param is set");
    let mut sys_args = env::args();
    
    if sys_args.len() == 1 {
        println!("No input and output argument set. Exiting. For help use `syncfolders ?´");
        return;
    }

    let _exec_arg = sys_args.next().clone();
    let first_arg = sys_args.next().clone();
    let second_arg = sys_args.next().clone();
    
    let as_question = Some(String::from("?"));
    if first_arg.eq(&as_question) {
        println!("Usage: syncfolders [FOLDER/TO/WATCH] [FOLDER/TO/SYNC]");
        return;
    }
    
    // watcher channel
    let (tx, rx) = channel();
    let (txw, rxw) = channel();
    let txx = txw.clone();

    println!("Starting sync process");

    let opt_in_path = first_arg.clone();
    let opt_out_path = second_arg.clone();

    
    
    
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

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    let in_path = first_arg.unwrap();
    watcher.watch(in_path, RecursiveMode::Recursive).unwrap();

    loop {
        
        if rx.try_recv().unwrap_or(0) == 1 { 
            println!("Closing."); 
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

fn map_to_complete_path(file_ref: &PathBuf, out_path: &PathBuf ) -> PathBuf  {
    let current_dir = std::env::current_dir().unwrap();
    return current_dir.join(&out_path).join(file_ref);
}

fn copy_to_output(in_file: &PathBuf, in_path: &str, out_path: &str ) {    
    let current_dir = std::env::current_dir().unwrap();

    let file_ref_buf = in_file.canonicalize().unwrap();
    let in_path_buf = current_dir.join(PathBuf::from(in_path)).canonicalize().unwrap();
    let out_path_buf = current_dir.join(PathBuf::from(out_path)).canonicalize().unwrap();
    
    let file_ref = get_relative_file_reference(&file_ref_buf, &in_path_buf);
    let write_out_path  = map_to_complete_path(&file_ref, &out_path_buf);

    match copy(&in_file, &write_out_path) {
        Ok(_) => { println!("File {:?} updated successfully", file_ref); },
        Err(e) => { 
            println!("Error copying file:"); 
            println!("Input: {:?}", in_file); 
            println!("Relateive path: {:?}", file_ref); 
            println!("Output: {:?}", write_out_path); 
            println!("Error: {:?}", e); 
        }
    };
}

fn remove_in_target(in_file: &PathBuf, in_path: &str, out_path: &str ) {    
    let current_dir = std::env::current_dir().unwrap();

    let out_path_buf = current_dir.join(PathBuf::from(out_path));
    let in_path_buf = current_dir.join(PathBuf::from(in_path));
    
    let file_ref = get_relative_file_reference_for_remove(&in_file, &in_path_buf);
    let write_out_path  = map_to_complete_path(&file_ref, &out_path_buf);
    
    match remove_file(&write_out_path) {
        Ok(_) => { println!("File {:?} deleted in target", file_ref); },
        Err(e) => { 
            println!("Error removing file:"); 
            println!("Input: {:?}", in_file); 
            println!("Relateive path: {:?}", file_ref); 
            println!("Output: {:?}", write_out_path); 
            println!("Error: {:?}", e); 
        }
    };
}

fn on_watch_event(evt: notify::DebouncedEvent, opt_in_path: &Option<std::string::String>, opt_out_path: &Option<std::string::String>) {
    let in_path = opt_in_path.clone().unwrap();
    let out_path = opt_out_path.clone().unwrap();

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