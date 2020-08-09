use std::str;
use std::thread;
use std::process::Command;
use crate::msg;
use crate::point;

pub struct Cron {
	pub alias: String,
	pub min: [u8; 60],
	pub hour: [u8; 24],
	pub dom: [u8; 32],	//first item is at idx 1
	pub mon: [u8; 13],
	pub dow: [u8; 7],
	pub cmd: String,
	pub trigger: bool,
}

impl Clone for Cron {
	fn clone(&self) -> Self {
		let mut c = Cron::new();
		c.alias = self.alias.clone();
		c.min.copy_from_slice(&self.min);
		c.hour.copy_from_slice(&self.hour);
		c.dom.copy_from_slice(&self.dom);
		c.mon.copy_from_slice(&self.mon);
		c.dow.copy_from_slice(&self.dow);
		c.cmd = self.cmd.clone();
		c.trigger = self.trigger;
		return c;
	}
}

impl Cron {
	pub fn new() -> Cron {
		Cron {
			alias: "".to_string(),
			min: [0;60], hour: [0;24], dom: [0;32], mon: [0;13], dow: [0;7],
			cmd: "".to_string(),
			trigger: false,
		}
	}

	pub fn process_line(&mut self, id: &[u8], l: &[u8]) -> Option<()> {
		let len = l.len();

		if len < 1 {
			return Some(());
		}

		if l[0] == '@' as u8 {
			//skip local config
			return Some(());
		}

		//# = remote config, send to server
		if l[0] == '#' as u8 {
			match msg::send_cfg(id,l) {
				Ok(_) => {
					return Some(());
				},
				Err(_) => {
					return None;
				},
			}
		}

		//get index of where the first word (alias) ends
		let cmd_idx = l.iter()
			.position(|c| c.is_ascii_whitespace())
			.unwrap();
		self.alias = String::from_utf8(l[..cmd_idx].to_vec()).unwrap();

		let mut iter = l.iter().skip(cmd_idx+1);

		let sch_idx;

		//see if schedule is "@"
		if l[cmd_idx+1] == '@' as u8 {
			//find end of word
			sch_idx = iter
				.position(|c| c.is_ascii_whitespace())
				.unwrap();
			let rebword = String::from_utf8(l[(cmd_idx+1)..sch_idx+cmd_idx+1].
				to_vec()).unwrap();
			match rebword.as_str() {
				"@reboot" => self.trigger = true,
				_ => {
					println!("ERROR: unknown schedule value {}",rebword);
					return None;
				},
			}
		} else {
			//only get first 5 tokens (cron schedule) for sending to server
			let (ntoks,idx) = iter.enumerate()
				.filter(|(_,c)| c.is_ascii_whitespace())
				.map(|(i,_)| i)
				.take(5)
				.fold((0,0),|acc,i| (acc.0+1,i));
			if ntoks < 5 {
				println!("ERROR: not enough tokens");
				return None;
			}

			let mut sch = msg::Sched::new();
			if let Err(e) = sch.parse(id, &l[(cmd_idx+1)..idx+cmd_idx+2]) {
				println!("ERROR: {}",e);
				return None;
			};
			self.min.copy_from_slice(&sch.min);
			self.hour.copy_from_slice(&sch.hour);
			self.dom.copy_from_slice(&sch.dom);
			self.mon.copy_from_slice(&sch.mon);
			self.dow.copy_from_slice(&sch.dow);

			sch_idx = idx + 1;
		}

		self.cmd = String::from_utf8(l[sch_idx+cmd_idx+1..].to_vec())
			.unwrap();

		Some(())
	}

	pub fn check_and_run(&mut self, id: &[u8], p: &point::Point) {
		//see if this command qualifies to run at this point
		if !self.trigger {
			let run = self.min[p.min as usize] + self.hour[p.hour as usize]
				+ self.dom[p.dom as usize] + self.mon[p.mon as usize]
				+ self.dow[p.dow as usize];
			if run != 5 {
				return;
			}
		} else {
			self.trigger = false;
		}
		println!("RUNNING {} {}",self.alias,self.cmd);
		let ccmd = self.clone();
		let mut cid = [0;16];
		cid.clone_from_slice(id);
		let cp = p.clone();
		thread::spawn(move || {
			let out = match Command::new("/bin/sh")
				.arg("-c")
				.arg(ccmd.cmd)
				.output() {
					Ok(o) => o,
					Err(e) => {
						println!("ERROR {}",e);
						return;
					},
			};
			//prepend timestamp to the output before sending
			let tstamped = format!("[{}]\n{}",cp,str::from_utf8(&out.stdout)
								   .unwrap());
			if let Err(e) = msg::send_cmd_out(&cid,&ccmd.alias,&tstamped) {
				println!("ERROR {}",e);
			};
		});
	}
}

