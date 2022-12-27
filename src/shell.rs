use core::ptr::null_mut;
use crate::{print, println};
use crate::vga_buf::SCREEN;
use pc_keyboard::DecodedKey;
use lazy_static::lazy_static;

const MAX_CHILDREN: usize = 10;
const MAX_DIR_NAME: usize = 10;

lazy_static! {
    static ref SH: spin::Mutex<Shell> = spin::Mutex::new({
        let mut sh = Shell::new();
        sh
    });
}

pub fn handle_keyboard_interrupt(key: DecodedKey) {
    match key {
        DecodedKey::Unicode(c) => SH.lock().on_key_pressed(c as u8),
        DecodedKey::RawKey(rk) => {}
    }
}

#[derive(Debug, Clone, Copy)]
struct Dir{
    index: usize,
    name: [u8; MAX_DIR_NAME],
    parent_index: usize,
    child_count: usize,
    child_indexes: [usize; MAX_CHILDREN]
}

struct Dirs{
    dirs:[Dir; 100],
    next_dir: usize,
}

struct Shell {
    buf: [u8; 80],
    buf_len: usize,
    dirs: Dirs,
    current_dir: Dir
}

impl Shell {

    pub fn new() -> Shell {
        let root_dir = Dir{
            index: 0,
            name: [b'r', b'o', b'o', b't', b' ', b' ', b' ', b' ', b' ', b' '],
            parent_index: 0,
            child_count: 0,
            child_indexes: [0; MAX_CHILDREN]
        };

        let mut dirs = Dirs{
            dirs: [Dir{
                index:0,
                name: [b' '; MAX_DIR_NAME],
                parent_index: 0,
                child_count: 0,
                child_indexes: [0; MAX_CHILDREN]
            }; 100],
            next_dir: 1
        };


        let mut shell = Shell {
            buf: [0; 80],
            buf_len: 0,
            dirs,
            current_dir: root_dir
        };

        shell.dirs.dirs[0] = shell.current_dir;

        return shell;
    }

    pub fn on_key_pressed(&mut self, key: u8) {
        match key {
            b'\n' => {
                let mut k = parse_input(self.buf, self.buf_len);

                if k.0[0] != b' '{
                    self.execute_command(k.0, k.1);
                }

                self.buf = [0; 80];
                self.buf_len = 0;
                println!()
            }
            _ => {
                self.buf[self.buf_len] = key;
                self.buf_len += 1;
                print!("{}", key as char);
            }
        }
    }

    pub fn execute_command(&mut self, command: [u8; 10], argument: [u8; 10]){
        println!();

        if compare_strings("cur_dir", command) {
            self.current_dir_command();
        }
        else if compare_strings("make_dir", command) {
            self.make_dir_command(argument);
        }
        else if compare_strings("change_dir", command) {
            self.change_dir_command(argument);
        }
        else if compare_strings("remove_dir", command){
            self.remove_dir_command(argument);
        }
        else if compare_strings("dir_tree", command) {
            self.dir_tree_command();
        }
        else if compare_strings("clear", command) {
            SCREEN.lock().clear();
        }
        else {
            Self::not_supported_command(command);
        }

    }

    pub fn not_supported_command(command: [u8;10]){
        print!("[error] Command '");

        for byte in command {
            if byte == b' ' {
                break;
            }
            print!("{}", byte as char);
        }

        print!("' is not supported.");
    }

    fn current_dir_command(&self){
        print!("/");
        write_array(self.current_dir.name);
    }

    fn make_dir_command(&mut self, dir_name: [u8; 10]){
        if dir_name[0] == b' '{
            print!("[error] The folder name is missing");
            return;
        }

        if self.find_childer_dir(self.current_dir.name, dir_name) != 0{
            print!("[error] Such dir is already exists!");
            return;
        }

        let new_dir_id = self.dirs.next_dir;

        if new_dir_id < 100{
            let parent_id = self.current_dir.index;

            let child = Dir {
                index: new_dir_id,
                name: dir_name,
                parent_index: parent_id,
                child_count: 0,
                child_indexes: [0; MAX_CHILDREN]
            };

            self.dirs.dirs[new_dir_id] = child;
            self.dirs.next_dir += 1;
            self.current_dir.child_indexes[self.current_dir.child_count] = child.index;
            self.current_dir.child_count += 1;
            self.dirs.dirs[self.current_dir.index] = self.current_dir;

            print!("[ok] The folder ");
            for byte in dir_name {
                print!("{}", byte as char);
            }
            print!("is created");
        }
    }

    fn change_dir_command(&mut self, dir_name: [u8; MAX_DIR_NAME]){
        if dir_name[0] == b' '{
            print!("[error] The folder name is missing");
            return;
        }

        if dir_name[0] == b'.'{
            if self.current_dir.index == 0 && self.current_dir.parent_index == 0{
                print!("[error] You are already in the root directory");
                return;
            }

            self.current_dir = self.dirs.dirs[self.current_dir.parent_index];
            print!("[ok] Directory changed");
            return;
        }

        let dir_id = self.find_childer_dir(self.current_dir.name, dir_name);

        if dir_id == 0{
            print!("[error] No such children directory!");
        }
        else {
            self.current_dir = self.dirs.dirs[dir_id];
            print!("[ok] Directory changed");
        }
    }

    fn remove_dir_command(&mut self, dir_name: [u8; 10]){
        if dir_name[0] == b' '{
            println!();
            print!("[error] The folder name is missing");
            return;
        }

        let dir_id = self.find_childer_dir(self.current_dir.name, dir_name);

        if dir_id == 0{
            println!();
            print!("[error] No such children directory!");
        }
        else {
            let mut i: usize = 0;
            for index in self.current_dir.child_indexes {
                if index == dir_id{
                    self.current_dir.child_indexes[i] = 0;
                    self.move_child_indexes(i);
                    break;
                }

                i += 1;
            }

            self.dirs.dirs[self.current_dir.index] = self.current_dir;

            self.remove_dir_by_index(dir_id);

            println!();
            print!("[ok] Directory ");
            write_array(dir_name);
            print!("was removed")
        }
    }

    fn dir_tree_command(&mut self){
        self.unwrap_dir(self.current_dir.index, 0);
    }

    fn find_childer_dir(&self, parent_dir: [u8; 10], new_dir: [u8; 10]) -> usize{
        let mut parent : Dir = Dir {
            index: 0,
            name: [b' '; MAX_CHILDREN],
            parent_index: 0,
            child_count: 0,
            child_indexes: [0;MAX_CHILDREN]
        };

        for i in 0..self.dirs.next_dir - 1 {
            if compare_array(parent_dir, (self.dirs.dirs[i] as Dir).name){
                parent = self.dirs.dirs[i];
                break;
            }
        }

        for i in parent.child_indexes{
            if compare_array(new_dir, (self.dirs.dirs[i] as Dir).name){
                return i;
            }
        }

        return 0;
    }

    fn move_child_indexes(&mut self, id: usize){
        let mut i = id + 1;
        self.current_dir.child_count -= 1;

        if i == MAX_CHILDREN{
            self.current_dir.child_indexes[id] = 0;
            return;
        }

        while i < MAX_CHILDREN {
            self.current_dir.child_indexes[i - 1] = self.current_dir.child_indexes[i];
            i += 1;
        }
    }

    fn remove_dir_by_index(&mut self, id: usize){
        self.dirs.dirs[id] = Dir{
            index: 0,
            name: [b' '; MAX_DIR_NAME],
            parent_index: 0,
            child_count: 0,
            child_indexes: [0; MAX_CHILDREN]
        };
    }

    fn unwrap_dir(&mut self, id: usize, level: u8){
        let mut i = 0;

        while i < level {
            print!("{}", "    ");
            i += 1;
        }

        if self.dirs.dirs[id].child_count == 0 {
            print!("/");
            write_array(self.dirs.dirs[id].name);
            println!();
        } else {
            print!("/");
            write_array(self.dirs.dirs[id].name);
            println!();

            let mut n = 0;
            while n < self.dirs.dirs[id].child_count {
                self.unwrap_dir(self.dirs.dirs[id].child_indexes[n], level + 1);
                n += 1;
            }
        }
    }
}

pub fn parse_input(buf: [u8; 80], buf_len: usize) -> ([u8; 10],[u8; 10]){
    let mut command: [u8; 10] = [b' '; 10];
    let mut argument: [u8; 10] = [b' '; 10];

    let mut i = 0;

    while buf[i] != b' ' && i < buf_len{
        if i >= 10 {
            println!();
            print!("[error] Command is to long");
            return ([b' '; 10], [b' '; 10]);
        }

        command[i] = buf[i];
        i += 1;
    }

    i += 1;
    let mut j = 0;

    while i < buf_len && buf[i] != b' '{
        if j >= 10 {
            println!();
            print!("[error] Directory name is too long");
            return ([b' '; 10], [b' '; 10]);
        }
        argument[j] = buf[i];
        i += 1;
        j += 1;
    }

    return (command, argument);
}

pub fn compare_strings(str: &str, arr: [u8; 10]) -> bool {
    let mut i = 0;
    let mut first: [u8; 10] = [b' '; 10];

    for byte in str.bytes(){
        first[i] = byte;
        i += 1;
    }

    return compare_array(first, arr);
}

pub fn compare_array(first: [u8; 10], second: [u8; 10]) -> bool {
    let mut is_correct = true;
    let mut i = 0;

    for byte in first{
        if byte != second[i]{
            is_correct = false;
            return is_correct;
        }

        i += 1;
    }

    return is_correct;
}

pub fn write_array(arr: [u8; 10]){
    for byte in arr{
        print!("{}", byte as char);
    }
}