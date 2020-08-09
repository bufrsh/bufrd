use std::io::prelude::*;
use std::net::TcpStream;
use std::io::Error;
use std::io::ErrorKind;
use std::path::Path;

use openssl::ssl::{SslMethod,SslConnector,SslFiletype};

use crate::cron;

const MAX_OUT: usize = 512;
const MAX_IN: usize = 512;
const HDR_LEN: usize = 20;
const ERRCODE: u8 = 0x52;
const VER:u8 = 0x01;
const TYPE_LOGIN: u8 = 0xa0;
const TYPE_SCHED: u8 = 0xa1;
const TYPE_STDOUT: u8 = 0xa2;
const TYPE_HBEAT: u8 = 0xa3;
const TYPE_CFG: u8 = 0xa4;

struct Msg<'a> {
	hdr: [u8; HDR_LEN], //LENL LENH VER TYPE ID0 ... ID15
	len: usize,
	pub body: Option<&'a [u8]>,
}

impl Msg<'_> {
	pub fn new(txbuf: Option<&[u8]>) -> Msg<'_> {
		let mut m = Msg {
			hdr: [0;HDR_LEN],
			body: txbuf,
			len: 0,
		};
		if let Some(tb) = txbuf {
			m.len = tb.len();
			if m.len > MAX_OUT {
				println!("Data length too big, trimming");
				m.len = MAX_OUT;
			}
			m.hdr[0] = (m.len & 0xff) as u8;
			m.hdr[1] = ((m.len & 0xff00) >> 8) as u8;
		};
		m.hdr[2] = VER;
		return m;
	}

	pub fn set_type(&mut self, t: u8) {
		self.hdr[3] = t;
	}

	pub fn set_id(&mut self, id: &[u8]) {
		self.hdr[4..HDR_LEN].copy_from_slice(id);
	}

	pub fn send(&mut self, res: &mut [u8]) -> Result<usize,Error> {
		let mut builder = SslConnector::builder(SslMethod::tls_client())
			.unwrap();
		builder.set_certificate_chain_file(Path::new("./cert.pem"))?;
		//PEM contains the CA cert. server's cert is signed with that too
		builder.set_ca_file(Path::new("./cert.pem"))?;
		builder.set_private_key_file(Path::new("./key.pem"),SslFiletype::PEM)?;
		let connector = builder.build();
		let stream = TcpStream::connect("server.bufr.sh:5000")?;
		let mut stream = match connector.connect("server.bufr.sh",stream) {
			Ok(s) => s,
			Err(_) => {
				return Err(Error::new(ErrorKind::InvalidInput,"Connect error"));
			},
		};
		//something goes wrong after this, daemon needs to restart
		if let Err(e) = stream.write(&self.hdr) {
			println!("ERR {}",e);
			std::process::exit(-1);
		};
		if let Some(b) = self.body {
			if let Err(e) = stream.write(&b[..self.len]) {
				println!("ERR {}",e);
				std::process::exit(-1);
			};
		};
		//read response
		if let Err(e) = stream.read(&mut self.hdr) {
			println!("ERR {}",e);
			std::process::exit(-1);
		};
		let dlen: usize = (self.hdr[0] as usize * 256)
			+ self.hdr[1] as usize;
		//anything bigger is definitely a bad response, quit here
		if dlen > MAX_IN {
			println!("Server sent bad header");
			std::process::exit(-1);
		}
		if let Err(e) = stream.read(&mut res[..dlen]) {
			println!("ERR {}",e);
			std::process::exit(-1);
		};
		if self.hdr[4] == ERRCODE {
			return Err(Error::new(ErrorKind::InvalidInput,
								  String::from_utf8(res[..dlen].to_vec())
								  .unwrap()));
		}

		Ok(dlen)
	}
}

pub struct Sched {
	pub min: [u8; 60],
	pub hour: [u8; 24],
	pub dom: [u8; 32],	//first item is at idx 1
	pub mon: [u8; 13],
	pub dow: [u8; 7],
}

impl Sched {
	pub fn new() -> Sched {
		Sched {
			min: [0;60],
			hour: [0;24],
			dom: [0;32],
			mon: [0;13],
			dow: [0;7],
		}
	}

	pub fn parse(&mut self, id: &[u8], l: &[u8]) -> Result<usize,Error> {
		let mut buf = [0;256];
		let mut m = Msg::new(Some(l));

		m.set_type(TYPE_SCHED);
		m.set_id(id);
		m.send(&mut buf)?;

		self.min[..60].copy_from_slice(&buf[..60]);
		self.hour[..24].copy_from_slice(&buf[60..84]);
		self.dom[..32].copy_from_slice(&buf[84..116]);
		self.mon[..13].copy_from_slice(&buf[116..129]);
		self.dow[..7].copy_from_slice(&buf[129..136]);

		return Ok(0);
	}
}

pub fn send_cmd_out(id: &[u8], alias: &str, out: &str) -> Result<i32,Error> {
	let mut rxbuf = [0;256];
	let delim = [TYPE_STDOUT];
	let txbuf = [alias.as_bytes(),&delim,out.as_bytes()].concat();
	let mut m = Msg::new(Some(&txbuf));

	m.set_type(TYPE_STDOUT);
	m.set_id(id);
	match m.send(&mut rxbuf) {
		Ok(n) => println!("{}",String::from_utf8(rxbuf[..n].to_vec()).
						  unwrap()),
		Err(e) => println!("ERROR {:?}",e),
	}

	Ok(0)
}

pub fn login(out: &mut [u8]) -> Option<()> {
	let mut m = Msg::new(None);

	m.set_type(TYPE_LOGIN);
	match m.send(out) {
		Ok(_) => {
			println!("Logged in");
			return Some(());
		},
		Err(e) => {
			println!("Login error {:?}",e);
			return None;
		},
	}
}

pub fn poll_triggers(id: &[u8], crons: &mut Vec<cron::Cron>) {
	let mut rxbuf = [0;MAX_IN];
	let mut m = Msg::new(None);

	m.set_type(TYPE_HBEAT);
	m.set_id(id);
	let dlen = m.send(&mut rxbuf);
	if let Err(_) = dlen {
		println!("No remote triggers");
		return
	}

	let dlen = dlen.unwrap();

	let resstr: String = rxbuf[..dlen].iter()
		.map(|&c| c as char)
		.collect();

	println!("TRIGGERS: {}",resstr);

	for cr in crons.iter_mut() {
		if let Some(w) = resstr.split_whitespace().find(|&s| s == cr.alias) {
			println!("TRIGGERED {}",w);
			cr.trigger = true;
		};
	}
}

pub fn send_cfg(id: &[u8], line: &[u8]) -> Result<usize,Error> {
	let mut rxbuf = [0;256];
	let mut m = Msg::new(Some(line));

	m.set_type(TYPE_CFG);
	m.set_id(id);
	match m.send(&mut rxbuf) {
		Ok(n) => {
			println!("{}",String::from_utf8(rxbuf[..n].to_vec()).unwrap());
			return Ok(0);
		},
		Err(e) => {
			println!("ERROR {:?}",e);
			return Err(e);
		},
	}
}

