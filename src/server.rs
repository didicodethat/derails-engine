use std::{
    sync::{mpsc::channel},
    thread::{self, sleep},
    time::Duration,
};

use crate::commons::{Settings};
use message_io::network::{NetEvent, Transport};
use message_io::node::{self};
use mlua::{Function, Lua, Value::{self}};

#[derive(Clone, Debug)]
enum LuaMessages {
    Tick(String)
}

pub fn start_server(settings: &Settings) {
    let address = format!("{}:{}", settings.game.address, settings.game.port);

    println!("Game Server Running on {}", &address);

    let (lua_send, lua_receive) = channel::<LuaMessages>();

    let lua_join_handle = thread::spawn(move || {
        let lua = init_lua();
        let tick = lua.globals().get::<_, Function>("onGameTick").unwrap();
        while let Ok(message) = lua_receive.recv() {
            match message {
                LuaMessages::Tick(message) => tick.call(message).unwrap(),
            }
        }
    });

    let network_join_handle = thread::spawn(move || {
        let (handler, listener) = node::split::<()>();
        handler.network().listen(Transport::Ws, address).unwrap();
        listener.for_each(move |event| match event.network() {
            NetEvent::Connected(_, _) => unreachable!(), // Used for explicit connections.
            NetEvent::Accepted(_endpoint, _listener) => println!("Client connected"), // Tcp or Ws
            NetEvent::Message(endpoint, data) => {
                println!("Received: {}", String::from_utf8_lossy(data));
                // endpoint.resource_id();
                // endpoint.addr();
                handler.network().send(endpoint, data);
            }
            NetEvent::Disconnected(_endpoint) => println!("Client disconnected"), //Tcp or Ws
        });
    });

    loop {
        if lua_join_handle.is_finished() {
            panic!("Lua Stopped Running, halting application");
        }

        if  network_join_handle.is_finished() {
            panic!("Networking stopped running, halting application");
        }

        lua_send
            .send(LuaMessages::Tick("hello From Tick :)".to_string()))
            .expect("Couldn't send a message to the lua engine");
        sleep(Duration::from_millis(33));
    }
}

fn init_lua() -> Lua {
    let lua = Lua::new();
    lua.load("require('main')")
        .exec()
        .expect("Couldn't execute the lua code");

    let required_callback_functions = ["onClientMessage", "onClientConnect", "onGameTick"];

    for fn_name in required_callback_functions {
        lua.globals()
            .get::<_, Function>(fn_name)
            .unwrap_or_else(|_| panic!("please create a global '{}' function", fn_name));
    }

    lua.load("print('Lua Loaded!')").eval::<Value>().expect("Couldn't print from lua");

    lua
}
