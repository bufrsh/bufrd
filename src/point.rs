use std::fmt;
use chrono::{Local,Timelike,Datelike};

#[derive(Debug)]
pub struct Point {
	pub min: u8,
	pub hour: u8,
	pub dom: u8,
	pub mon: u8,
	pub dow: u8,
}

impl Point {
	pub fn now() -> Point {
		let dt = Local::now();
		Point {
			min: dt.minute() as u8,
			hour: dt.hour() as u8,
			dom: dt.day() as u8,
			mon: dt.month() as u8,
			dow: (dt.weekday().number_from_sunday() - 1) as u8,
		}
	}

	pub fn get_secs() -> u64 {
		let dt = Local::now();
		dt.second() as u64
	}
}

impl fmt::Display for Point {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let dowstr = match self.dow {
			0 => "Sun",
			1 => "Mon",
			2 => "Tue",
			3 => "Wed",
			4 => "Thu",
			5 => "Fri",
			6 => "Sat",
			_ => "WTF",
		};
		let monstr = match self.mon {
			1 => "Jan",
			2 => "Feb",
			3 => "Mar",
			4 => "Apr",
			5 => "May",
			6 => "Jun",
			7 => "Jul",
			8 => "Aug",
			9 => "Sep",
			10 => "Oct",
			11 => "Nov",
			12 => "Dec",
			_ => "LOL",
		};
		write!(f,"{} {} {} {:02}:{:02}",dowstr,monstr,self.dom,self.hour,
			   self.min)
	}
}

impl Clone for Point {
	fn clone(&self) -> Self {
		Point {
			min: self.min,
			hour: self.hour,
			dom: self.dom,
			mon: self.mon,
			dow: self.dow,
		}
	}
}

