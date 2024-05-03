

pub fn main(){
    let mut ctx = zmq::Context::new();
    let zero_url = "tcp://127.0.0.1:5555";
    let socket = ctx.socket(zmq::SUB).unwrap();
    socket.connect(zero_url).unwrap();
    let filter = b"";
    socket.set_subscribe(filter);
    let mut msg = zmq::Message::new();
    loop {
        socket.recv(&mut msg, 0).unwrap();
        println!("msg: {}", msg.as_str().unwrap());
    }
}