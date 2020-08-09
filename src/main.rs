use std::fs::File;
use std::io::{BufRead,BufReader};
use std::process::exit;
use std::thread;
use std::time::Duration;
use std::env;

mod msg;
mod point;
mod cron;
mod cert;
mod conf;

fn load_cron(id: &[u8], crons: &mut Vec<cron::Cron>) -> Option<()> {
	let f = File::open("./bufr.conf").unwrap();
	let reader = BufReader::new(f);
	for line in reader.lines() {
		let l = line.unwrap();
		let mut c = cron::Cron::new();
		match c.process_line(id,l.as_bytes()) {
			Some(()) => {
			},
			None => return None,
		}
		crons.push(c);
	}
	return Some(());
}

fn main()
{
	let args: Vec<String> = env::args().collect();
	if args.len() > 1 {
		match &args[1][..] {
			"gen" => {
				let mut cnf = conf::Conf::new();
				match cnf.gen() {
					Some(_) => println!("Conf gen OK"),
					None => println!("Conf gen ERR"),
				}
				return;
			},
			_ => {
				println!("Unknown arg");
				return;
			},
		}
	}

	match cert::check() {
		(cert::CertStatus::OK,_) => println!("Certificate OK"),
		(cert::CertStatus::NOCERT,_) => {
			println!("Requesting new certificate");
			if cert::request(None) == None {
				std::process::exit(-1);
			}
		},
		(cert::CertStatus::EXPIRED,hash) => {
			println!("Requesting certificate renewal");
			if cert::request(hash) == None {
				std::process::exit(-1);
			}
		},
		(cert::CertStatus::ERR,_) => {
			println!("Certificate status check failed");
			std::process::exit(-1);
		},
	}

	let mut id = [0;16];
	if msg::login(&mut id) == None {
		std::process::exit(-1);
	}

	let mut crons: Vec<cron::Cron> = Vec::new();
	match load_cron(&id,&mut crons) {
		Some(()) => println!("{}","cron load OK"),
		None => {
			println!("{}","cron load FAIL");
			exit(-2);
		}
	}

	loop {
		//make sure we wait exactly until next minute mark
		let waitsec = 60 - point::Point::get_secs();
		thread::sleep(Duration::from_secs(waitsec));
		msg::poll_triggers(&id,&mut crons);
		let p = point::Point::now();
		println!("{}",p);
		for c in crons.iter_mut() {
			c.check_and_run(&id,&p);
		}
	}
}

