// Minimal multiplayer client logic
// Opens a WebSocket, broadcasts local player position, updates remote players.

let ws;
let api = {};
let playerId = Math.random().toString(36).slice(2, 10);
let connected = false;

function connect(url) {
  ws = new WebSocket(url);
  ws.onopen = () => {
    connected = true;
    console.log('[MP] connected');
    // Announce join
    send({ t: 'join', id: playerId });
  };
  ws.onclose = () => {
    console.log('[MP] disconnected');
    connected = false;
  };
  ws.onerror = (e) => console.log('[MP] error', e);
  ws.onmessage = (ev) => {
    try {
      const msg = JSON.parse(ev.data);
      handleMessage(msg);
    } catch (e) {
      console.warn('[MP] invalid message', ev.data);
    }
  };
}

function send(obj) {
  if (ws && connected) {
    ws.send(JSON.stringify(obj));
  }
}

function handleMessage(msg) {
  switch (msg.t) {
    case 'join':
      if (msg.id !== playerId) {
        api.doom_add_remote_player(msg.id, msg.x || 0, msg.y || 0);
      }
      break;
    case 'leave':
      api.doom_remove_remote_player(msg.id);
      break;
    case 'pos':
      if (msg.id !== playerId) {
        api.doom_update_remote_player(msg.id, msg.x, msg.y);
      }
      break;
  }
}

function broadcastPosition() {
  if (!connected) return;
  try {
    const arr = api.doom_get_player_position();
    const x = arr[0];
    const y = arr[1];
    send({ t: 'pos', id: playerId, x, y });
  } catch (e) {
    // ignore
  }
}

export function initMultiplayer(a) {
  api = a;
  // Provide a global to allow manual connect from console: MP_CONNECT('ws://localhost:8081')
  window.MP_CONNECT = (url) => connect(url);
  window.MP_ID = () => playerId;
  setInterval(broadcastPosition, 200); // 5 updates/sec
}
