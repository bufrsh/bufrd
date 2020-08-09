use std::fmt::Write;
use std::string::String;
use std::path::Path;
use std::fs;
use openssl::x509::X509;
use openssl::asn1::Asn1Time;
use openssl::ec::EcKey;
use openssl::nid::Nid;
use openssl::x509::X509ReqBuilder;
use openssl::pkey::PKey;
use openssl::hash::MessageDigest;
use reqwest;
use reqwest::StatusCode;
use crate::conf;

pub enum CertStatus {
	OK,
	NOCERT,
	EXPIRED,
	ERR,
}

pub fn check() -> (CertStatus,Option<String>) {
	if !Path::new("./cert.pem").exists() {
		println!("Cert doesn't exist");
		return (CertStatus::NOCERT,None)
	}
	let fstr = fs::read_to_string("./cert.pem").unwrap();
	let f = fstr.as_bytes();
	let pem = X509::from_pem(f).unwrap();
	let expires = pem.not_after();
	println!("Cert expires on {}",expires);

	let today = Asn1Time::days_from_now(0).unwrap();
	let rem = today.diff(&expires).unwrap();

	if rem.secs <= 0 {
		println!("Cert is expired");
		//save expired fingerprint for sending to server for replacement
		let fingerprint = match pem.digest(MessageDigest::sha1()) {
			Ok(digest) => {
				let mut s = String::new();
				for b in digest.iter() {
					write!(&mut s,"{:02x}",b).expect("digest error");
				}
				s
			},
			Err(e) => {
				println!("ERR {}",e);
				return (CertStatus::ERR,None);
			},
		};
		return (CertStatus::EXPIRED,Some(fingerprint));
	}

	return (CertStatus::OK,None)
}

fn send_csr_get_cert(csr: &str, hash: Option<String>) -> Option<()> {
	let mut cnf = conf::Conf::new();
	if let None = cnf.read() {
		return None;
	}

	let client = reqwest::blocking::ClientBuilder::new()
		.danger_accept_invalid_certs(true)
		.build().unwrap();

	let res = match hash {
		Some(h) => {
			client.post("https://bufr.sh/devcert")
			.form(&[("uname", cnf.user.as_str()),
				("pass", cnf.pass.as_str()),("csr", csr), ("dev", &h)])
			.send().unwrap()
		},
		None => {
			client.post("https://bufr.sh/devcert")
			.form(&[("uname", cnf.user.as_str()),
				("pass", cnf.pass.as_str()), ("csr", csr)])
			.send().unwrap()
		},
	};

	match res.status() {
		StatusCode::OK => {
			println!("Got cert");
			fs::write("./cert.pem",res.text().unwrap()).unwrap();
			return Some(())
		},
		s => {
			println!("ERR {:?}",s);
			return None
		}
	}
}

pub fn request(hash: Option<String>) -> Option<()> {
	//Generate EC key pair
	let key = EcKey::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
	let keypair = EcKey::generate(key.group()).unwrap();

	fs::write("./key.pem",keypair.private_key_to_pem().unwrap())
		.unwrap();

	//Generate CSR for sending to server
	let mut reqbldr = X509ReqBuilder::new().unwrap();
	let pkey = PKey::from_ec_key(keypair).unwrap();
	reqbldr.set_pubkey(&pkey).unwrap();
	reqbldr.sign(&pkey,MessageDigest::sha256()).unwrap();

	let req = reqbldr.build();

	return send_csr_get_cert(&String::from_utf8(req.to_pem().unwrap())
		.unwrap(),hash)
}

