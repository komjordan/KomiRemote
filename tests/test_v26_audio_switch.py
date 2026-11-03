"""Targeted std-only harness: executes extracted production lifecycle methods.
No Cargo, native device, Flutter or functional call test.
"""
from pathlib import Path
import os
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
CLIENT = ROOT / 'src/client/io_loop.rs'
SERVER = ROOT / 'src/server/connection.rs'

def method(path, name):
    text = path.read_text()
    start = text.rfind('\n', 0, text.index('fn ' + name)) + 1
    return balanced(text, start)

def balanced(text, start):
    brace = text.index('{', start)
    depth = 1
    end = brace + 1
    while depth:
        depth += (text[end] == '{') - (text[end] == '}')
        end += 1
    return text[start:end]

def arm(path, anchor):
    text = path.read_text()
    block = balanced(text, text.index(anchor))
    return block[block.index('{') + 1:-1]

class AudioSwitch(unittest.TestCase):
    def test_acceptance_and_response_source_guards(self):
        client = CLIENT.read_text()
        # Both acceptance directions must keep an existing transferred sender.
        answer = client.split('Data::KomiAnswerVoiceCall(accept) => {', 1)[1].split('Data::CloseVoiceCall', 1)[0]
        self.assertIn('if self.stop_voice_call_sender.is_none()', answer)
        response = client.split('Some(message::Union::VoiceCallResponse(response)) => {', 1)[1].split('Some(message::Union::PeerInfo', 1)[0]
        self.assertIn('if self.stop_voice_call_sender.is_none()', response)
        self.assertLess(response.index('response.req_timestamp != ts.get()'), response.index('self.voice_call_request_timestamp = None'))
        host = method(SERVER, 'handle_voice_call')
        self.assertLess(host.index('if let Some(ts)'), host.index('self.komi_cut_video_for_voice()'))
        host = method(SERVER, 'komi_handle_voice_call_response')
        self.assertLess(host.index('ts != req_timestamp'), host.index('self.komi_outgoing_voice_call = None'))
        cleanup = method(CLIENT, 'handle_disconnected')
        self.assertIn('self.komi_stop_video_audio()', cleanup)
        self.assertIn('self.stop_voice_call()', cleanup)
        self.assertIn('self.voice_call_request_timestamp = None', cleanup)
        video = (ROOT / 'src/komi_video_call.rs').read_text()
        predicate = balanced(video, video.index('fn live_video'))
        self.assertIn('if rt.machine.is_show_camera()', predicate)
        self.assertLess(predicate.index('return false'), predicate.index('Phase::Active'))

    def test_extracted_host_lifecycle(self):
        methods = '\n'.join(method(SERVER, name) for name in [
            'komi_cut_video_for_voice', 'komi_stop_video_audio', 'komi_sync_video_audio',
            'komi_acquire_microphone', 'komi_release_microphone', 'komi_voice_timestamp',
            'handle_voice_call', 'komi_handle_voice_call_response', 'close_voice_call',
            'komi_check_outgoing_voice_call'])
        leases = '\n'.join(method(CLIENT, name) for name in ['komi_acquire_microphone', 'komi_release_microphone'])
        source = r'''
#![allow(dead_code,unused_imports)]
use std::{num::NonZeroI64, sync::{Arc,Weak,RwLock,atomic::{AtomicBool,AtomicUsize,Ordering}}};
static ACTIVE:AtomicBool=AtomicBool::new(false);
static OPENS:AtomicUsize=AtomicUsize::new(0);
static STOPS:AtomicUsize=AtomicUsize::new(0);
static CANCEL:std::sync::atomic::AtomicI64=std::sync::atomic::AtomicI64::new(0);
extern crate self as log;
#[macro_export] macro_rules! warn {($($x:tt)*)=>{()}}
static KOMI_MICROPHONE_USERS:std::sync::Mutex<usize>=std::sync::Mutex::new(0);
LEASES
fn komi_voice_cancel(ts:i64) {CANCEL.store(ts,Ordering::SeqCst);}
mod client {pub mod io_loop {pub(crate) use crate::{komi_acquire_microphone,komi_release_microphone,komi_voice_cancel};}}
mod komi_video_call {
 use super::*;
 pub fn host_active_video(_:i32)->bool {ACTIVE.load(Ordering::SeqCst)}
 pub fn host_video_busy(_:i32)->bool {ACTIVE.load(Ordering::SeqCst)}
 pub fn host_command(_:i32,_:&str,_:bool) {ACTIVE.store(false,Ordering::SeqCst);}
}
mod audio_service {pub const NAME:&str="audio"; pub fn set_voice_call_input_device(d:Option<String>,_:bool){if d.is_some(){super::OPENS.fetch_add(1,super::Ordering::SeqCst);}else{super::STOPS.fetch_add(1,super::Ordering::SeqCst);}}}
fn get_default_sound_input()->Option<String>{Some("mic".into())}
fn new_voice_call_request(_:bool) {}
fn new_voice_call_response(_:i64,_:bool) {}
enum Data {CloseVoiceCall(String),StartVoiceCall}
mod host {
 use super::*;
 #[derive(Clone)] struct Inner;
 impl Inner {fn id(&self)->i32{1}}
 struct Server;
 impl Server {fn subscribe(&mut self,_:&str,_:Inner,_:bool){}}
 pub struct Conn {voice_calling:bool,video_call_audio:bool,voice_call_request_timestamp:Option<NonZeroI64>,komi_outgoing_voice_call:Option<(std::time::Instant,i64)>,komi_active_voice_timestamp:Option<i64>,komi_microphone_owned:bool,inner:Inner,server:Weak<RwLock<Server>>,closed:usize,sent:usize}
 impl Conn {
 fn new()->Self{Self{voice_calling:false,video_call_audio:false,voice_call_request_timestamp:None,komi_outgoing_voice_call:None,komi_active_voice_timestamp:None,komi_microphone_owned:false,inner:Inner,server:Weak::new(),closed:0,sent:0}}
 fn komi_video_bind(&self){}
 fn audio_enabled(&self)->bool{true}
 fn is_authed_view_camera_conn(&self)->bool{false}
 fn send_to_cm(&mut self,_:Data){self.closed+=1;}
 async fn send(&mut self,_:()){self.sent+=1;}
 METHODS
 }
 #[test] fn host_ringing_survives_video_ticks() {
 let mut c=Conn::new(); ACTIVE.store(true,Ordering::SeqCst); super::block_on(c.komi_sync_video_audio());
 c.voice_call_request_timestamp=NonZeroI64::new(3);c.komi_outgoing_voice_call=Some((std::time::Instant::now(),4));
 for _ in 0..100 {super::block_on(c.komi_sync_video_audio());}
 assert!(c.voice_call_request_timestamp.is_some(),"host incoming ringing killed");assert!(c.komi_outgoing_voice_call.is_some(),"host outgoing ringing killed");assert_eq!(c.closed,0);assert_eq!(c.sent,0);
 ACTIVE.store(false,Ordering::SeqCst);super::block_on(c.komi_sync_video_audio());
 }
 #[test] fn host_voice_to_video_to_voice_never_stops_mic() {
 STOPS.store(0,Ordering::SeqCst);OPENS.store(0,Ordering::SeqCst);
 let mut c=Conn::new();c.voice_calling=true;c.komi_acquire_microphone();OPENS.store(0,Ordering::SeqCst);c.komi_active_voice_timestamp=Some(2);ACTIVE.store(true,Ordering::SeqCst);
 super::block_on(c.komi_sync_video_audio()); assert!(!c.voice_calling);assert!(c.video_call_audio);assert_eq!(STOPS.load(Ordering::SeqCst),0);assert_eq!(OPENS.load(Ordering::SeqCst),0);assert_eq!(c.sent,0);
 c.komi_cut_video_for_voice();assert!(c.voice_calling);assert!(!c.video_call_audio);assert_eq!(STOPS.load(Ordering::SeqCst),0);
 c.voice_calling=false;c.komi_release_microphone();
 }
 #[test] fn multi_connection_release_keeps_other_microphone() {
 OPENS.store(0,Ordering::SeqCst);STOPS.store(0,Ordering::SeqCst);
 let mut a=Conn::new();let mut b=Conn::new();a.komi_acquire_microphone();b.komi_acquire_microphone();
 assert_eq!(OPENS.load(Ordering::SeqCst),1);
 a.komi_release_microphone();assert_eq!(STOPS.load(Ordering::SeqCst),0);
 b.komi_release_microphone();assert_eq!(STOPS.load(Ordering::SeqCst),1);
 b.komi_release_microphone();assert_eq!(STOPS.load(Ordering::SeqCst),1);
 }
 #[test] fn stale_host_accept_does_not_cut_video_or_consume_current_timestamp() {
 let mut c=Conn::new();ACTIVE.store(true,Ordering::SeqCst);super::block_on(c.komi_sync_video_audio());
 super::block_on(c.handle_voice_call(true));assert!(ACTIVE.load(Ordering::SeqCst));
 c.komi_outgoing_voice_call=Some((std::time::Instant::now(),21));
 super::block_on(c.komi_handle_voice_call_response(true,20));
 assert_eq!(c.komi_outgoing_voice_call.unwrap().1,21);assert!(ACTIVE.load(Ordering::SeqCst));
 super::block_on(c.komi_handle_voice_call_response(false,21));assert!(ACTIVE.load(Ordering::SeqCst));
 c.voice_call_request_timestamp=NonZeroI64::new(22);
 super::block_on(c.handle_voice_call(false));assert!(ACTIVE.load(Ordering::SeqCst));
 c.komi_outgoing_voice_call=Some((std::time::Instant::now()-std::time::Duration::from_secs(31),23));
 super::block_on(c.komi_check_outgoing_voice_call());assert!(c.komi_outgoing_voice_call.is_none());assert_eq!(CANCEL.load(Ordering::SeqCst),23);assert!(ACTIVE.load(Ordering::SeqCst));
 super::block_on(c.komi_handle_voice_call_response(true,23));assert!(!c.voice_calling);
 ACTIVE.store(false,Ordering::SeqCst);super::block_on(c.komi_sync_video_audio());
 }
 #[test] fn concurrent_late_worker_releases_cannot_reset_retained_capture() {
 OPENS.store(0,Ordering::SeqCst);STOPS.store(0,Ordering::SeqCst);
 komi_acquire_microphone();let mut workers=Vec::new();
 for _ in 0..32 {workers.push(std::thread::spawn(|| {for _ in 0..100 {komi_acquire_microphone();std::thread::yield_now();komi_release_microphone();}}));}
 for worker in workers {worker.join().unwrap();}
 assert_eq!(OPENS.load(Ordering::SeqCst),1);assert_eq!(STOPS.load(Ordering::SeqCst),0);
 komi_release_microphone();assert_eq!(STOPS.load(Ordering::SeqCst),1);
 }
 #[test] fn valid_host_voice_accept_preserves_video_microphone_and_hangup_releases_once() {
 for incoming in [false,true] {
 let mut c=Conn::new();ACTIVE.store(true,Ordering::SeqCst);super::block_on(c.komi_sync_video_audio());STOPS.store(0,Ordering::SeqCst);OPENS.store(0,Ordering::SeqCst);
 if incoming {c.voice_call_request_timestamp=NonZeroI64::new(31);super::block_on(c.handle_voice_call(true));}
 else {c.komi_outgoing_voice_call=Some((std::time::Instant::now(),31));super::block_on(c.komi_handle_voice_call_response(true,31));}
 assert!(!ACTIVE.load(Ordering::SeqCst));assert!(c.voice_calling);assert_eq!(c.komi_active_voice_timestamp,Some(31));assert_eq!(STOPS.load(Ordering::SeqCst),0);assert_eq!(OPENS.load(Ordering::SeqCst),0);
 super::block_on(c.close_voice_call());assert_eq!(STOPS.load(Ordering::SeqCst),1);assert!(c.komi_voice_timestamp().is_none());
 super::block_on(c.close_voice_call());assert_eq!(STOPS.load(Ordering::SeqCst),1);
 }
 }
}
fn block_on<F:std::future::Future>(f:F)->F::Output {
 struct Wake;impl std::task::Wake for Wake{fn wake(self:Arc<Self>){}}
 let w=std::task::Waker::from(Arc::new(Wake));let mut cx=std::task::Context::from_waker(&w);let mut f=std::pin::pin!(f);
 match f.as_mut().poll(&mut cx){std::task::Poll::Ready(x)=>x,_=>panic!("unexpected IO")}
}
'''.replace(' METHODS', methods).replace('LEASES', leases)
        self.run_rust(source)

    def run_rust(self, source):
        scratch = Path(os.environ.get('TMPDIR', '/home/hermes/.hermes/cache/scratch'))
        with tempfile.TemporaryDirectory(prefix='v26-host-audio-', dir=scratch) as tmp:
            path = Path(tmp) / 'audio.rs'
            binary = Path(tmp) / 'audio-test'
            path.write_text(source)
            subprocess.run(['rustc', '--edition=2021', '--test', str(path), '-o', str(binary)], check=True)
            subprocess.run([str(binary), '--test-threads=1', '--nocapture'], check=True)

    def test_extracted_controller_lifecycle(self):
        methods = '\n'.join(method(CLIENT, name) for name in [
            'stop_voice_call', 'komi_session', 'komi_stop_video_audio',
            'komi_cut_video_for_voice', 'komi_sync_video_audio'])
        adapters = '\n'.join(
            f'async fn {name}(&mut self, {arg}) -> bool {{ {arm(CLIENT, anchor)} true }}'
            for name, arg, anchor in [
                ('response', 'response:Response', 'Some(message::Union::VoiceCallResponse(response)) =>'),
                ('request', 'request:Request', 'Some(message::Union::VoiceCallRequest(request)) =>'),
                ('answer', 'accept:bool, peer:&mut Peer', 'Data::KomiAnswerVoiceCall(accept) =>'),
                ('close', 'peer:&mut Peer', 'Data::CloseVoiceCall =>'),
                ('new_voice', 'peer:&mut Peer', 'Data::NewVoiceCall =>'),
            ])
        harness = r'''
#![allow(dead_code, unused_imports)]
use std::{num::NonZeroI64, sync::{mpsc::{channel, Sender}, atomic::{AtomicBool, Ordering}}};
extern crate self as log;
#[macro_export] macro_rules! debug {($($x:tt)*)=>{()}}
macro_rules! allow_err {($x:expr)=>{let _ = $x;}}
static ACTIVE: AtomicBool = AtomicBool::new(false);
static BUSY: AtomicBool = AtomicBool::new(false);
static LAST_SENT:std::sync::atomic::AtomicI64=std::sync::atomic::AtomicI64::new(0);
fn get_time()->i64{100}
#[derive(Default)] struct VoiceCallRequest {req_timestamp:i64,is_connect:bool}
type Request=VoiceCallRequest;
#[derive(Default)] struct Message {request:VoiceCallRequest}
impl Message {
 fn new()->Self {Self::default()}
 fn set_voice_call_request(&mut self,r:VoiceCallRequest){self.request=r;}
 fn voice_call_request(&self)->&VoiceCallRequest{&self.request}
}
CANCEL_METHOD
struct Response {req_timestamp:i64,accepted:bool}
struct Peer;
impl Peer {async fn send(&mut self,m:&Message)->Result<(),()>{LAST_SENT.store(m.request.req_timestamp,Ordering::SeqCst);Ok(())}}
mod client {pub fn new_voice_call_response(_:i64,_:bool)->super::Message{super::Message::new()}}
enum Data { Message(Message) }
mod komi_video_call {
    use super::*;
    pub fn controller_active_video(_: &str)->bool { ACTIVE.load(Ordering::SeqCst) }
    pub fn controller_video_busy(_: &str)->bool { BUSY.load(Ordering::SeqCst) }
    pub fn controller_bind(_: &str, _: Sender<Data>) {}
    pub fn controller_command(_: &str, _: &str) { ACTIVE.store(false,Ordering::SeqCst); BUSY.store(false,Ordering::SeqCst); }
}
#[derive(Default)] struct Handler { closed: usize, started:usize, incoming:usize }
impl Handler {
 fn on_voice_call_closed(&mut self, _: &str) {self.closed+=1;}
 fn on_voice_call_started(&mut self) {self.started+=1;}
 fn on_voice_call_incoming(&mut self) {self.incoming+=1;}
 fn on_voice_call_waiting(&mut self) {}
}
struct Remote {
    sender: Sender<Data>, handler: Handler,
    stop_voice_call_sender: Option<Sender<()>>, video_audio_stop: Option<Sender<()>>,
    voice_call_request_timestamp: Option<NonZeroI64>, komi_incoming_voice_call: Option<i64>,
    komi_video_audio_active: bool, komi_active_voice_timestamp: Option<i64>,
    komi_last_voice_timestamp:i64,
    starts: usize,
}
impl Remote {
    fn new()->Self {Self{sender:channel().0, handler:Handler::default(), stop_voice_call_sender:None,video_audio_stop:None,voice_call_request_timestamp:None,komi_incoming_voice_call:None,komi_video_audio_active:false,komi_active_voice_timestamp:None,komi_last_voice_timestamp:0,starts:0}}
    fn start_voice_call(&mut self)->Option<Sender<()>> {self.starts+=1;Some(channel().0)}
    METHODS
    ADAPTERS
}
fn block_on<F:std::future::Future>(f:F)->F::Output {
    struct Wake; impl std::task::Wake for Wake {fn wake(self:std::sync::Arc<Self>) {}}
    let w=std::task::Waker::from(std::sync::Arc::new(Wake));
    let mut cx=std::task::Context::from_waker(&w); let mut f=std::pin::pin!(f);
    match f.as_mut().poll(&mut cx) {std::task::Poll::Ready(x)=>x,_=>panic!("unexpected async IO")}
}
#[test] fn ring_during_video_survives_ticks_in_both_directions() {
    let mut r=Remote::new(); ACTIVE.store(true,Ordering::SeqCst);
    block_on(r.komi_sync_video_audio()); assert_eq!(r.starts,1);
    r.voice_call_request_timestamp=NonZeroI64::new(101);
    r.komi_incoming_voice_call=Some(102);
    for _ in 0..100 {block_on(r.komi_sync_video_audio());}
    assert_eq!(r.voice_call_request_timestamp.unwrap().get(),101,"outgoing ringing killed by video tick");
    assert_eq!(r.komi_incoming_voice_call,Some(102),"incoming ringing killed by video tick");
    assert_eq!(r.handler.closed,0); assert_eq!(r.starts,1);
}
#[test] fn accepted_video_transfers_live_sender_once_without_cancel_signal() {
    let mut r=Remote::new(); let (tx,rx)=channel(); r.stop_voice_call_sender=Some(tx);
    let (data_tx,data_rx)=channel(); r.sender=data_tx;
    r.komi_active_voice_timestamp=Some(12);
    ACTIVE.store(true,Ordering::SeqCst); block_on(r.komi_sync_video_audio());
    assert!(r.stop_voice_call_sender.is_none()); assert!(r.video_audio_stop.is_some());
    assert_eq!(r.starts,0); assert_eq!(rx.try_recv(),Err(std::sync::mpsc::TryRecvError::Empty));
    assert!(data_rx.try_recv().is_err(),"stale vocal cancellation must not follow video acceptance");
    block_on(r.komi_sync_video_audio()); assert_eq!(r.handler.closed,1);
    BUSY.store(true,Ordering::SeqCst); r.komi_cut_video_for_voice();
    assert!(r.stop_voice_call_sender.is_some()); assert!(r.video_audio_stop.is_none());
    block_on(r.komi_sync_video_audio()); assert_eq!(r.starts,0);
    assert_eq!(rx.try_recv(),Err(std::sync::mpsc::TryRecvError::Empty));
    r.stop_voice_call(); assert_eq!(rx.try_recv(),Ok(()));
}
#[test] fn controller_accept_both_directions_reuses_transferred_sender() {
 for incoming in [false,true] {
 let mut r=Remote::new();let (tx,rx)=channel();r.video_audio_stop=Some(tx);r.komi_video_audio_active=true;ACTIVE.store(true,Ordering::SeqCst);BUSY.store(true,Ordering::SeqCst);
 if incoming {r.komi_incoming_voice_call=Some(31);block_on(r.answer(true,&mut Peer));}
 else {r.voice_call_request_timestamp=NonZeroI64::new(31);block_on(r.response(Response{req_timestamp:31,accepted:true}));}
 assert!(!ACTIVE.load(Ordering::SeqCst));assert!(r.stop_voice_call_sender.is_some());assert!(r.video_audio_stop.is_none());assert_eq!(r.starts,0);assert_eq!(r.komi_active_voice_timestamp,Some(31));assert_eq!(r.handler.started,1);
 assert_eq!(rx.try_recv(),Err(std::sync::mpsc::TryRecvError::Empty));
 block_on(r.close(&mut Peer));assert_eq!(rx.try_recv(),Ok(()));assert!(r.komi_active_voice_timestamp.is_none());
 }
}
#[test] fn timestamp_mismatch_duplicate_and_stale_cancel_do_not_kill_new_call() {
 let mut r=Remote::new();r.voice_call_request_timestamp=NonZeroI64::new(51);
 block_on(r.response(Response{req_timestamp:50,accepted:true}));assert_eq!(r.voice_call_request_timestamp.unwrap().get(),51);
 block_on(r.request(Request{req_timestamp:50,is_connect:false}));assert!(r.voice_call_request_timestamp.is_some());
 block_on(r.close(&mut Peer));assert!(r.voice_call_request_timestamp.is_none());
 block_on(r.response(Response{req_timestamp:51,accepted:true}));assert_eq!(r.starts,0);
 r.komi_active_voice_timestamp=Some(52);let(tx,rx)=channel();r.stop_voice_call_sender=Some(tx);
 block_on(r.request(Request{req_timestamp:51,is_connect:false}));assert!(r.stop_voice_call_sender.is_some());assert_eq!(rx.try_recv(),Err(std::sync::mpsc::TryRecvError::Empty));
 block_on(r.request(Request{req_timestamp:52,is_connect:false}));assert_eq!(rx.try_recv(),Ok(()));assert!(r.komi_active_voice_timestamp.is_none());
}
#[test] fn outgoing_ids_are_monotonic_across_clock_stall_and_direction_switch() {
 let mut r=Remote::new();let mut p=Peer;
 block_on(r.new_voice(&mut p));assert_eq!(r.voice_call_request_timestamp.unwrap().get(),100);
 block_on(r.close(&mut p));assert_eq!(LAST_SENT.load(Ordering::SeqCst),100);
 block_on(r.new_voice(&mut p));assert_eq!(r.voice_call_request_timestamp.unwrap().get(),101);
 block_on(r.close(&mut p));block_on(r.request(Request{req_timestamp:1000,is_connect:true}));block_on(r.answer(false,&mut p));
 block_on(r.new_voice(&mut p));assert_eq!(r.voice_call_request_timestamp.unwrap().get(),1001);
 block_on(r.close(&mut p));r.komi_last_voice_timestamp=i64::MAX;block_on(r.new_voice(&mut p));assert!(r.voice_call_request_timestamp.is_none());
}
#[test] fn refuse_timeout_and_show_camera_do_not_stop_video_microphone() {
 let mut r=Remote::new();let(tx,rx)=channel();r.video_audio_stop=Some(tx);r.komi_video_audio_active=true;ACTIVE.store(true,Ordering::SeqCst);BUSY.store(true,Ordering::SeqCst);
 r.voice_call_request_timestamp=NonZeroI64::new(61);block_on(r.response(Response{req_timestamp:61,accepted:false}));assert!(ACTIVE.load(Ordering::SeqCst));assert!(r.video_audio_stop.is_some());
 block_on(r.request(Request{req_timestamp:62,is_connect:true}));block_on(r.answer(false,&mut Peer));assert!(ACTIVE.load(Ordering::SeqCst));assert!(r.video_audio_stop.is_some());
 block_on(r.request(Request{req_timestamp:63,is_connect:true}));block_on(r.request(Request{req_timestamp:63,is_connect:false}));assert!(r.komi_incoming_voice_call.is_none());assert!(r.video_audio_stop.is_some());assert_eq!(rx.try_recv(),Err(std::sync::mpsc::TryRecvError::Empty));
 ACTIVE.store(false,Ordering::SeqCst);block_on(r.komi_sync_video_audio());assert_eq!(rx.try_recv(),Ok(()));
 let mut show_camera=Remote::new();BUSY.store(false,Ordering::SeqCst);block_on(show_camera.komi_sync_video_audio());assert_eq!(show_camera.starts,0);
}
'''.replace('    METHODS', methods).replace('    ADAPTERS', adapters).replace('CANCEL_METHOD', method(CLIENT, 'komi_voice_cancel'))
        scratch = Path(os.environ.get('TMPDIR', '/home/hermes/.hermes/cache/scratch'))
        with tempfile.TemporaryDirectory(prefix='v26-audio-', dir=scratch) as tmp:
            source = Path(tmp) / 'audio.rs'
            binary = Path(tmp) / 'audio-test'
            source.write_text(harness)
            subprocess.run(['rustc', '--edition=2021', '--test', str(source), '-o', str(binary)], check=True)
            subprocess.run([str(binary), '--test-threads=1', '--nocapture'], check=True)

if __name__ == '__main__':
    unittest.main(verbosity=2)
