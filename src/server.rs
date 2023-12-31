use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web_actors::ws;
// use printers;
// use std::fs::File;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use crate::endecrypt_file;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
#[derive(Debug, PartialEq, Eq)]
enum TranserStatus{
    STARTED,
    FINISHED,
    NOOP

}
/// websocket connection is long running connection, it easier
/// to handle with an actor
pub struct MyWebSocket {
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    hb: Instant,
    status: TranserStatus,
    filename: String
}

impl MyWebSocket {
    pub fn new() -> Self {
        Self { hb: Instant::now(), status: TranserStatus::NOOP, filename: String::from("foo.txt") }
    }

    /// helper method that sends ping to client every 5 seconds (HEARTBEAT_INTERVAL).
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Actor for MyWebSocket {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }
}

/// Handler for `ws::Message`
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        // process websocket messages
        println!("WS: {msg:?}");
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                println!("received str: {}",text);
                // Vector of system printers
                // let printers = printers::get_printers();
                // println!("{:?}", printers);
                if text.starts_with("filename:") {
                    if self.status == TranserStatus::STARTED {
                        ctx.text(format!("echo: {} transfer is ongoing",self.filename));
                    }
                    let filename = text.replace("filename:","");
                    self.status =  TranserStatus::STARTED;
                    self.filename= filename;
                } else if text.eq("finished") {
                    self.status = TranserStatus::FINISHED;
                    let _ = endecrypt_file::decrypt_file(&self.filename);
                }
                
                
                ctx.text(format!("{}",text))
            },
            Ok(ws::Message::Binary(bin)) =>  { 
                // let mut file = File::create("foo.txt");
                // let _ = fs::write("foo.txt", &bin);
                println!("received bin data");
                let mut data_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.filename)
                .expect("cannot open file");

                // Write to a file
                data_file
                    .write(&bin)
                    .expect("write failed");
                // ctx.binary(bin)
            },
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}
