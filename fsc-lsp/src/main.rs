mod diagnostics;
mod documents;
mod line_index;
mod notifications;
mod session;

use crate::notifications::handle_notification;
use crate::session::Session;
use lsp_server::{Connection, ErrorCode, Message, Notification, Response};
use lsp_types::InitializeParams;
use lsp_types::notification::{Notification as _, PublishDiagnostics};
use lsp_types::{
    PositionEncodingKind, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions,
};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .try_init()?;
    log::info!("starting FSC language server");

    let (connection, io_threads) = Connection::stdio();
    let capabilities = ServerCapabilities {
        position_encoding: Some(PositionEncodingKind::UTF16),
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                ..TextDocumentSyncOptions::default()
            },
        )),
        ..ServerCapabilities::default()
    };

    let initialize_params = connection.initialize(serde_json::to_value(capabilities)?)?;
    let initialize_params = serde_json::from_value::<InitializeParams>(initialize_params)?;
    let mut session = Session::new(&initialize_params);

    main_loop(&connection, &mut session)?;
    drop(connection);
    io_threads.join()?;
    log::info!("shutting down server");
    Ok(())
}
fn main_loop(
    connection: &Connection,
    session: &mut Session,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    break;
                }
                //TODO request handling
                let response = Response::new_err(
                    request.id,
                    ErrorCode::MethodNotFound as i32,
                    format!("unsupported request `{}`", request.method),
                );
                connection.sender.send(Message::Response(response))?;
            }
            Message::Notification(notification) => {
                let method = notification.method.clone();
                match handle_notification(notification, session) {
                    Ok(Some(params)) => {
                        let notification =
                            Notification::new(PublishDiagnostics::METHOD.to_owned(), params);
                        connection
                            .sender
                            .send(Message::Notification(notification))?;
                    }
                    Ok(None) => {}
                    Err(error) => log::warn!("ignoring {method} notification: {error}"),
                }
            }
            Message::Response(_) => {}
        }
    }
    Ok(())
}
