use std::collections::VecDeque;
use std::io;
use std::mem::{self, MaybeUninit};
use std::net::SocketAddr;
use std::sync::Arc;

use flume::{Receiver, Sender};
use futures_lite::{
    io::{AsyncRead, AsyncWrite},
    FutureExt,
};
use hashbrown::HashMap;
use mqtt_proto::{
    v3::{
        Connect, ConnectReturnCode, Header, Packet, PollPacketState, Publish, Subscribe,
        SubscribeReturnCode, Unsubscribe,
    },
    Error, Pid, Protocol, QoS, QosPid,
};
use tokio::sync::oneshot;

use crate::hook::{
    HookConnectedAction, HookReceipt, HookRequest, LockedHookContext, PublishAction,
    SubscribeAction, UnsubscribeAction,
};
use crate::protocols::mqtt::{
    BroadcastPackets, OnlineLoop, OnlineSession, PendingPackets, WritePacket,
};
use crate::state::{
    ClientId, ClientReceiver, ControlMessage, Executor, GlobalState, NormalMessage,
};

use super::{
    packet::{
        common::{after_handle_packet, handle_pendings, write_packet},
        connect::{handle_connect, handle_disconnect},
        publish::{
            handle_puback, handle_pubcomp, handle_publish, handle_pubrec, handle_pubrel,
            recv_publish, send_publish, RecvPublish, SendPublish,
        },
        subscribe::{handle_subscribe, handle_unsubscribe},
    },
    Session, SessionState,
};

#[allow(clippy::too_many_arguments)]
pub async fn handle_connection<T: AsyncRead + AsyncWrite + Unpin, E: Executor>(
    conn: T,
    peer: SocketAddr,
    header: Header,
    protocol: Protocol,
    timeout_receiver: Receiver<()>,
    hook_requests: Sender<HookRequest>,
    executor: E,
    global: Arc<GlobalState>,
) -> io::Result<()> {
    match handle_online(
        conn,
        peer,
        header,
        protocol,
        timeout_receiver,
        &hook_requests,
        &executor,
        &global,
    )
    .await
    {
        Ok(Some((session, receiver))) => {
            log::info!(
                "executor {:03}, {} go to offline, total {} clients ({} online)",
                executor.id(),
                peer,
                global.clients_count(),
                global.online_clients_count(),
            );
            executor.spawn_local(handle_offline(session, receiver, global));
        }
        Ok(None) => {
            log::info!(
                "executor {:03}, {} finished, total {} clients ({} online)",
                executor.id(),
                peer,
                global.clients_count(),
                global.online_clients_count(),
            );
        }
        Err(err) => {
            log::info!(
                "executor {:03}, {} error: {}, total {} clients ({} online)",
                executor.id(),
                peer,
                err,
                global.clients_count(),
                global.online_clients_count(),
            );
            return Err(err);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_online<T: AsyncRead + AsyncWrite + Unpin, E: Executor>(
    mut conn: T,
    peer: SocketAddr,
    _header: Header,
    protocol: Protocol,
    timeout_receiver: Receiver<()>,
    hook_requests: &Sender<HookRequest>,
    executor: &E,
    global: &Arc<GlobalState>,
) -> io::Result<Option<(Session, ClientReceiver)>> {
    let mut session = Session::new(&global.config, peer);
    let mut receiver = None;

    let packet = match Connect::decode_with_protocol(&mut conn, protocol)
        .or(async {
            log::info!("connection timeout: {}", peer);
            let _ = timeout_receiver.recv_async().await;
            Err(Error::IoError(io::ErrorKind::TimedOut, String::new()))
        })
        .await
    {
        Ok(packet) => packet,
        Err(err) => {
            log::debug!("mqtt v3.x connect codec error: {}", err);
            return Err(io::ErrorKind::InvalidData.into());
        }
    };
    drop(timeout_receiver);

    // Run before connect hook
    if global.config.hook.enable_before_connect {
        before_connect_hook(peer, &packet, hook_requests).await?;
    }

    let session_present = handle_connect(
        &mut session,
        &mut receiver,
        packet,
        &mut conn,
        executor,
        global,
    )
    .await?;

    if !session.connected {
        log::info!("{} not connected", session.peer);
        return Err(io::ErrorKind::InvalidData.into());
    }

    // Run after connect hook
    if global.config.hook.enable_before_connect {
        after_connect_hook(&mut session, session_present, hook_requests, global).await?;
    }

    for packet in after_handle_packet(&mut session) {
        write_packet(session.client_id, &mut conn, &packet).await?;
    }

    let receiver = receiver.expect("receiver");
    log::info!(
        "executor {:03}, {} connected, total {} clients ({} online) ",
        executor.id(),
        session.peer,
        global.clients_count(),
        global.online_clients_count(),
    );

    let mut taken_over = false;
    let online_loop = OnlineLoop::new(
        &mut session,
        global,
        &receiver,
        receiver.control.stream(),
        receiver.normal.stream(),
        hook_requests.sink(),
        &mut conn,
        &mut taken_over,
        PollPacketState::default(),
    );
    let io_error = online_loop.await;
    if taken_over {
        return Ok(None);
    }

    // FIXME: check all place depend on session.disconnected
    if !session.disconnected {
        log::debug!("[{}] handling will...", session.client_id);
        handle_will(&mut session, global).await?;
    }
    for (target_id, info) in session.broadcast_packets.drain() {
        for msg in info.msgs {
            if let Err(err) = info
                .sink
                .sender()
                .send_async((session.client_id, msg))
                .await
            {
                log::warn!(
                    "[{}] handle will, send broadcast message to {} failed: {:?}",
                    session.client_id,
                    target_id,
                    err
                )
            }
        }
    }
    if session.clean_session {
        global.remove_client(session.client_id, session.subscribes().keys());
        if let Some(err) = io_error {
            return Err(err);
        }
    } else {
        // become a offline client, but session keep updating
        global.offline_client(session.client_id);
        session.connected = false;
        return Ok(Some((session, receiver)));
    }

    Ok(None)
}

impl OnlineSession for Session {
    type Packet = Packet;
    type Error = Error;
    type SessionState = SessionState;

    fn client_id(&self) -> ClientId {
        self.client_id
    }
    fn disconnected(&self) -> bool {
        self.disconnected
    }
    fn build_state(&mut self, receiver: ClientReceiver) -> Self::SessionState {
        let mut pending_packets = PendingPackets::new(0, 0, 0);
        let mut qos2_pids = HashMap::new();
        let mut subscribes = HashMap::new();
        let mut broadcast_packets = HashMap::new();
        mem::swap(&mut self.pending_packets, &mut pending_packets);
        mem::swap(&mut self.qos2_pids, &mut qos2_pids);
        mem::swap(&mut self.subscribes, &mut subscribes);
        mem::swap(&mut self.broadcast_packets, &mut broadcast_packets);
        SessionState {
            client_id: self.client_id,
            receiver,
            protocol: self.protocol,

            server_packet_id: self.server_packet_id,
            pending_packets,
            qos2_pids,
            subscribes,
            broadcast_packets_cnt: self.broadcast_packets_cnt,
            broadcast_packets,
        }
    }

    fn consume_broadcast(&mut self, count: usize) {
        self.broadcast_packets_cnt -= count;
    }
    fn broadcast_packets_cnt(&self) -> usize {
        self.broadcast_packets_cnt
    }
    fn broadcast_packets_max(&self) -> usize {
        self.broadcast_packets_max
    }
    fn broadcast_packets(&mut self) -> &mut HashMap<ClientId, BroadcastPackets> {
        &mut self.broadcast_packets
    }

    fn handle_decode_error(
        &mut self,
        err: Self::Error,
        _write_packets: &mut VecDeque<WritePacket<Self::Packet>>,
    ) -> Result<(), Option<io::Error>> {
        log::debug!("[{}] mqtt v3.x codec error: {}", self.client_id, err);
        if err.is_eof() {
            if !self.disconnected() {
                Err(Some(io::ErrorKind::UnexpectedEof.into()))
            } else {
                Err(None)
            }
        } else {
            Err(Some(io::ErrorKind::InvalidData.into()))
        }
    }

    fn handle_packet(
        &mut self,
        encode_len: usize,
        packet_body: Vec<MaybeUninit<u8>>,
        packet: Self::Packet,
        write_packets: &mut VecDeque<WritePacket<Self::Packet>>,
        global: &Arc<GlobalState>,
    ) -> Result<Option<(HookRequest, oneshot::Receiver<HookReceipt>)>, Option<io::Error>> {
        if encode_len > global.config.max_packet_size_server as usize {
            log::debug!(
                "packet too large, size={}, max={}",
                encode_len,
                global.config.max_packet_size_server
            );
            return Err(Some(io::ErrorKind::InvalidData.into()));
        }
        match packet {
            Packet::Disconnect => handle_disconnect(self),
            Packet::Publish(pkt) => {
                if global.config.hook.enable_publish {
                    let locked_hook_context = LockedHookContext::new(self, write_packets);
                    let (hook_sender, hook_receiver) = oneshot::channel();
                    let hook_request = HookRequest::V3Publish {
                        context: locked_hook_context,
                        encode_len,
                        packet_body,
                        publish: pkt,
                        sender: hook_sender,
                    };
                    return Ok(Some((hook_request, hook_receiver)));
                } else if let Some(packet) = handle_publish(self, pkt, global).map_err(Some)? {
                    write_packets.push_back(packet.into());
                }
            }
            Packet::Puback(pid) => handle_puback(self, pid),
            Packet::Pubrec(pid) => write_packets.push_back(handle_pubrec(self, pid).into()),
            Packet::Pubrel(pid) => write_packets.push_back(handle_pubrel(self, pid)?.into()),
            Packet::Pubcomp(pid) => handle_pubcomp(self, pid),
            Packet::Subscribe(pkt) => {
                if global.config.hook.enable_subscribe {
                    let locked_hook_context = LockedHookContext::new(self, write_packets);
                    let (hook_sender, hook_receiver) = oneshot::channel();
                    let hook_request = HookRequest::V3Subscribe {
                        context: locked_hook_context,
                        encode_len,
                        packet_body,
                        subscribe: pkt,
                        sender: hook_sender,
                    };
                    return Ok(Some((hook_request, hook_receiver)));
                } else {
                    let retain_packets = handle_subscribe(self, &pkt, global)?;
                    write_packets.extend(retain_packets.into_iter().map(WritePacket::Packet));
                }
            }
            Packet::Unsubscribe(pkt) => {
                if global.config.hook.enable_unsubscribe {
                    let locked_hook_context = LockedHookContext::new(self, write_packets);
                    let (hook_sender, hook_receiver) = oneshot::channel();
                    let hook_request = HookRequest::V3Unsubscribe {
                        context: locked_hook_context,
                        encode_len,
                        packet_body,
                        unsubscribe: pkt,
                        sender: hook_sender,
                    };
                    return Ok(Some((hook_request, hook_receiver)));
                } else {
                    write_packets.push_back(handle_unsubscribe(self, &pkt, global).into());
                }
            }
            Packet::Pingreq => {
                log::debug!("{} received a ping packet", self.client_id);
                write_packets.push_back(Packet::Pingresp.into())
            }
            _ => {
                log::info!(
                    "[{}] received a invalid packet: {:?}",
                    self.client_id,
                    packet
                );
                return Err(Some(io::ErrorKind::InvalidData.into()));
            }
        }
        Ok(None)
    }

    fn after_handle_packet(&mut self, write_packets: &mut VecDeque<WritePacket<Self::Packet>>) {
        let pending_packets = after_handle_packet(self);
        write_packets.extend(pending_packets.into_iter().map(WritePacket::Packet));
    }

    fn handle_control(
        &mut self,
        msg: ControlMessage,
        _global: &Arc<GlobalState>,
    ) -> (bool, Option<Sender<SessionState>>) {
        handle_control(self, msg)
    }

    fn handle_normal(
        &mut self,
        sender: ClientId,
        msg: NormalMessage,
        _global: &Arc<GlobalState>,
    ) -> Option<(QoS, Option<Self::Packet>)> {
        handle_normal(self, sender, msg)
    }

    fn handle_pendings(&mut self) -> Vec<Packet> {
        handle_pendings(self)
    }
}

async fn handle_offline(mut session: Session, receiver: ClientReceiver, _global: Arc<GlobalState>) {
    loop {
        tokio::select! {
            result = receiver.control.recv_async() => match result {
                Ok(msg) => {
                    let (stop, sender_opt) = handle_control(&mut session, msg);
                    if let Some(sender) = sender_opt {
                        let old_state = session.build_state(receiver);
                        if let Err(err) = sender.send_async(old_state).await {
                            log::warn!("offline send session state failed: {err:?}");
                        }
                        break;
                    }
                    if stop {
                        break;
                    }
                }
                Err(err) => {
                    log::warn!("offline client receive control message error: {:?}", err);
                    break;
                }
            },
            result = receiver.normal.recv_async() => match result {
                Ok((sender, msg)) => {
                    let _ =  handle_normal(&mut session, sender, msg);
                }
                Err(err) => {
                    log::warn!("offline client receive normal message error: {:?}", err);
                    break;
                }
            }
        }
    }
    log::debug!("offline client finished: {:?}", session.client_id());
}

#[inline]
async fn handle_will(session: &mut Session, global: &Arc<GlobalState>) -> io::Result<()> {
    if let Some(last_will) = session.last_will.take() {
        let encode_len = {
            let qos_pid = match last_will.qos {
                QoS::Level0 => QosPid::Level0,
                QoS::Level1 => QosPid::Level1(Default::default()),
                QoS::Level2 => QosPid::Level2(Default::default()),
            };
            let publish = Publish {
                dup: false,
                retain: false,
                qos_pid,
                topic_name: last_will.topic_name.clone(),
                payload: last_will.message.clone(),
            };
            Packet::Publish(publish)
                .encode_len()
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?
        };
        send_publish(
            session,
            SendPublish {
                topic_name: &last_will.topic_name,
                retain: last_will.retain,
                qos: last_will.qos,
                payload: &last_will.message,
                encode_len,
            },
            global,
        );
    }
    Ok(())
}

/// return if the offline client loop should stop
#[inline]
fn handle_control(
    session: &mut Session,
    msg: ControlMessage,
) -> (bool, Option<Sender<SessionState>>) {
    // FIXME: call receiver.try_recv() to clear the channel, if the pending
    // queue is full, set a marker to the global state so that the sender stop
    // sending qos0 messages to this client.
    let mut stop = false;
    match msg {
        ControlMessage::OnlineV3 { sender } => return (false, Some(sender)),
        ControlMessage::OnlineV5 { .. } => {
            log::info!("take over v3.x by v5.x client is not allowed");
        }
        ControlMessage::Kick { reason } => {
            log::info!(
                "kick \"{}\", reason: {}, online: {}",
                session.client_id,
                reason,
                !session.disconnected,
            );
            stop = !session.disconnected;
        }
        ControlMessage::WillDelayReached { .. } | ControlMessage::SessionExpired { .. } => {
            unreachable!();
        }
    }
    (stop, None)
}

/// return if the offline client loop should stop
#[inline]
fn handle_normal(
    session: &mut Session,
    sender: ClientId,
    msg: NormalMessage,
) -> Option<(QoS, Option<Packet>)> {
    match msg {
        NormalMessage::PublishV3 {
            ref topic_name,
            qos,
            retain,
            ref payload,
            ref subscribe_filter,
            subscribe_qos,
            encode_len: _,
        } => {
            log::debug!(
                "{:?} received a v3.x publish message from {:?}",
                session.client_id,
                sender
            );
            recv_publish(
                session,
                RecvPublish {
                    topic_name,
                    qos,
                    retain,
                    payload,
                    subscribe_filter,
                    subscribe_qos,
                },
            )
        }
        NormalMessage::PublishV5 {
            ref topic_name,
            qos,
            retain,
            ref payload,
            ref subscribe_filter,
            subscribe_qos,
            properties: _,
            encode_len: _,
        } => {
            log::debug!(
                "{:?} received a v5.x publish message from {:?}",
                session.client_id,
                sender
            );
            recv_publish(
                session,
                RecvPublish {
                    topic_name,
                    qos,
                    retain,
                    payload,
                    subscribe_filter,
                    subscribe_qos,
                },
            )
        }
    }
}

async fn before_connect_hook(
    peer: SocketAddr,
    packet: &Connect,
    hook_requests: &Sender<HookRequest>,
) -> io::Result<()> {
    let (hook_tx, hook_rx) = oneshot::channel();
    let hook_request = HookRequest::V3BeforeConnect {
        peer,
        connect: packet.clone(),
        sender: hook_tx,
    };
    if let Err(err) = hook_requests.send_async(hook_request).await {
        log::error!("No hook service found: {err:?}");
        return Err(io::ErrorKind::InvalidData.into());
    }
    let code = match hook_rx.await {
        Ok(resp) => resp?.to_v3_code(),
        Err(err) => {
            log::error!("Hook service stopped: {err:?}");
            return Err(io::ErrorKind::InvalidData.into());
        }
    };
    if code != ConnectReturnCode::Accepted {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(())
}

async fn after_connect_hook(
    session: &mut Session,
    session_present: bool,
    hook_requests: &Sender<HookRequest>,
    global: &Arc<GlobalState>,
) -> io::Result<()> {
    let locked_hook_context = LockedHookContext::new(session, &mut Default::default());
    let (hook_tx, hook_rx) = oneshot::channel();
    let hook_request = HookRequest::V3AfterConnect {
        context: locked_hook_context,
        session_present,
        sender: hook_tx,
    };
    if let Err(err) = hook_requests.send_async(hook_request).await {
        log::error!("No hook service found: {err:?}");
        return Err(io::ErrorKind::InvalidData.into());
    }
    let actions = match hook_rx.await {
        Ok(resp) => resp?,
        Err(err) => {
            log::error!("Hook service stopped: {err:?}");
            return Err(io::ErrorKind::InvalidData.into());
        }
    };
    for action in actions {
        match action {
            HookConnectedAction::Publish(PublishAction {
                retain,
                qos,
                topic_name,
                payload,
                ..
            }) => {
                let encode_len = {
                    let qos_pid = match qos {
                        QoS::Level0 => QosPid::Level0,
                        QoS::Level1 => QosPid::Level1(Default::default()),
                        QoS::Level2 => QosPid::Level2(Default::default()),
                    };
                    let publish = Publish {
                        dup: false,
                        retain,
                        qos_pid,
                        topic_name: topic_name.clone(),
                        payload: payload.clone(),
                    };
                    Packet::Publish(publish).encode_len().map_err(|_| {
                        log::error!("action publish message too large");
                        io::Error::from(io::ErrorKind::InvalidData)
                    })?
                };
                send_publish(
                    session,
                    SendPublish {
                        qos,
                        retain,
                        topic_name: &topic_name,
                        payload: &payload,
                        encode_len,
                    },
                    global,
                );
            }
            HookConnectedAction::Subscribe(SubscribeAction(topics)) => {
                let subscribe = Subscribe::new(Pid::default(), topics.clone());
                match handle_subscribe(session, &subscribe, global) {
                    Ok(packets) => match &packets[0] {
                        Packet::Suback(suback) => {
                            for reason_code in &suback.topics {
                                match reason_code {
                                    SubscribeReturnCode::MaxLevel0
                                    | SubscribeReturnCode::MaxLevel1
                                    | SubscribeReturnCode::MaxLevel2 => {}
                                    code => {
                                        log::error!("action subscribe message error return code: {:?}, topics={:?}", code, topics,);
                                        break;
                                    }
                                }
                            }
                        }
                        _ => log::error!("action subscribe message invalid (retain included)"),
                    },
                    Err(err) => log::error!("action subscribe message invalid: {:?}", err),
                }
            }
            HookConnectedAction::Unsubscribe(UnsubscribeAction(topics)) => {
                let unsubscribe = Unsubscribe::new(Pid::default(), topics);
                let _unsuback = handle_unsubscribe(session, &unsubscribe, global);
            }
        }
    }
    Ok(())
}
