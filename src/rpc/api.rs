use std::net::SocketAddr;
use std::sync::mpsc::channel;
use std::sync::Arc;

use serde_json::Value;

use super::super::process::{Error as ProcessError, Message as ProcessMessage};
use super::super::types::HandlerContext;
use super::router::Router;
use super::types::{response, AgentGetInfoResponse, CodeChainCallRPCResponse, RPCResult, ShellStartCodeChainRequest};
use rpc::types::RPCError;
use rpc::types::ERR_NETWORK_ERROR;

pub fn add_routing(router: &mut Router) {
    router.add_route("ping", Box::new(ping as fn(Arc<HandlerContext>) -> RPCResult<String>));
    router.add_route(
        "shell_startCodeChain",
        Box::new(shell_start_codechain as fn(Arc<HandlerContext>, (ShellStartCodeChainRequest,)) -> RPCResult<()>),
    );
    router.add_route("shell_stopCodeChain", Box::new(shell_stop_codechain as fn(Arc<HandlerContext>) -> RPCResult<()>));
    router.add_route(
        "shell_getCodeChainLog",
        Box::new(shell_get_codechain_log as fn(Arc<HandlerContext>) -> RPCResult<String>),
    );
    router.add_route(
        "agent_getInfo",
        Box::new(agent_get_info as fn(Arc<HandlerContext>) -> RPCResult<AgentGetInfoResponse>),
    );
    router.add_route(
        "codechain_callRPC",
        Box::new(
            codechain_call_rpc as fn(Arc<HandlerContext>, (String, Vec<Value>)) -> RPCResult<CodeChainCallRPCResponse>,
        ),
    )
}

fn ping(_context: Arc<HandlerContext>) -> RPCResult<String> {
    response("pong".to_string())
}

fn shell_start_codechain(context: Arc<HandlerContext>, req: (ShellStartCodeChainRequest,)) -> RPCResult<()> {
    let (req,) = req;
    cinfo!("{}", req.env);
    cinfo!("{}", req.args);

    let (tx, rx) = channel();
    context.process.send(ProcessMessage::Run {
        env: req.env,
        args: req.args,
        callback: tx,
    })?;
    let process_result = rx.recv()?;
    process_result?;
    response(())
}

fn shell_stop_codechain(context: Arc<HandlerContext>) -> RPCResult<()> {
    let (tx, rx) = channel();
    context.process.send(ProcessMessage::Stop {
        callback: tx,
    })?;
    let process_result = rx.recv()?;
    process_result?;
    response(())
}

fn shell_get_codechain_log(context: Arc<HandlerContext>) -> RPCResult<String> {
    let (tx, rx) = channel();
    context.process.send(ProcessMessage::GetLog {
        callback: tx,
    })?;
    let process_result = rx.recv()?;
    let result = process_result?;
    response(result)
}

fn agent_get_info(context: Arc<HandlerContext>) -> RPCResult<AgentGetInfoResponse> {
    let (tx, rx) = channel();
    context.process.send(ProcessMessage::GetStatus {
        callback: tx,
    })?;
    let process_result = rx.recv()?;
    let (node_status, port) = process_result?;
    let ip_address = context.codechain_address.ip();
    let default_port = context.codechain_address.port();
    response(AgentGetInfoResponse {
        status: node_status,
        address: SocketAddr::new(
            ip_address,
            if port == 0 {
                default_port
            } else {
                port
            },
        ),
    })
}

fn codechain_call_rpc(context: Arc<HandlerContext>, args: (String, Vec<Value>)) -> RPCResult<CodeChainCallRPCResponse> {
    let (method, arguments) = args;
    let (tx, rx) = channel();
    context.process.send(ProcessMessage::CallRPC {
        method,
        arguments,
        callback: tx,
    })?;
    let process_result = rx.recv()?;
    let value = match process_result {
        Ok(value) => value,
        Err(ProcessError::CodeChainRPC(_)) => {
            return Err(RPCError::ErrorResponse(ERR_NETWORK_ERROR, "Network Error".to_string(), None))
        }
        Err(err) => return Err(err.into()),
    };
    response(CodeChainCallRPCResponse {
        inner_response: value,
    })
}
