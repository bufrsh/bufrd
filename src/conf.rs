use std::fmt::Write as FmtWrite;
use std::fs;
use std::fs::File;
use std::io::{BufRead,BufReader};
use std::io::{stdin, stdout, Write};
use openssl::hash::{hash,MessageDigest};

pub struct Conf {
	pub user: String,
	pub pass: String,
	pub name: String,
}

impl Conf {
	pub fn new() -> Conf {
		Conf {
			user: "".to_string(),
			pass: "".to_string(),
			name: "".to_string(),
		}
	}

	fn to_string(&self) -> Option<String> {
		if self.name.len()==0 || self.user.len()==0 || self.pass.len()==0 {
			return None;
		}
		let mut all = format!("@USER {}\n@PASS {}\n#NAME {}\n",self.user,
			self.pass,self.name);
		//Put some sample crons so user knows immediately everything works
		all.push_str("TestCmd * * * * * date\n");
		all.push_str("HelloCmd */2 * * * * echo Hello World!\n");
		return Some(all);
	}

	pub fn read(&mut self) -> Option<()> {
		let f = File::open("./bufr.conf")
			.expect("Please generate bufr.conf file using `gen` argument");
		let reader = BufReader::new(f);
		for line in reader.lines() {
			let lstr = line.unwrap();
			let l = lstr.as_bytes();
			//only concerned with local config lines (that start with an @)
			if l[0] != '@' as u8 {
				continue;
			}

			let idx = l.iter()
				.position(|c| c.is_ascii_whitespace())
				.unwrap();
			let word = String::from_utf8(l[..idx].to_vec()).unwrap();
			let val = String::from_utf8(l[idx+1..].to_vec()).unwrap();

			match word.as_str() {
				"@USER" => {
					self.user = val;
				},
				"@PASS" => {
					self.pass = val;
				},
				_ => {
					println!("Unknown conf word");
					return None;
				}
			}
		}

		if self.user.len() != 0 && self.pass.len() != 0 {
			return Some(());
		}

		println!("user/pass not read");
		return None;
	}

	fn write(&self) -> Option<()> {
		if let Some(s) = self.to_string() {
			fs::write("./bufr.conf",s).unwrap();
		}
		return Some(());
	}

	pub fn gen(&mut self) -> Option<()> {
		let mut inname: String = String::from("");
		let mut inuser: String = String::from("");
		let mut inpass: String = String::from("");
		print!("Enter device name: ");
		stdout().flush().unwrap();
		stdin().read_line(&mut inname).unwrap();
		self.name = inname.trim().to_string();
		print!("Enter username: ");
		stdout().flush().unwrap();
		stdin().read_line(&mut inuser).unwrap();
		self.user = inuser.trim().to_string();
		print!("Enter password: ");
		stdout().flush().unwrap();
		//should use password input here someday
		stdin().read_line(&mut inpass).unwrap();
		inpass = inpass.trim().to_string();
		self.pass = match hash(MessageDigest::sha1(),inpass.as_bytes()) {
			Ok(digest) => {
				let mut s = String::new();
				for b in digest.iter() {
					write!(&mut s,"{:02x}",b).expect("digest error");
				}
				s
			},
			Err(e) => {
				println!("ERR {}",e);
				return None;
			},
		};
		return self.write();
	}
}

