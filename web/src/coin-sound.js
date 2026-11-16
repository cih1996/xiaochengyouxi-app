let context;
let buffer;
let loading;

// Resume directly from the user's tap so delayed video rewards can play too.
export async function prepareCoinSound() {
  context ??= new AudioContext();
  const audioContext = context;
  const resumed = context.resume();
  loading ??= fetch("/assets/coin-arrival.wav")
    .then((response) => {
      if (!response.ok)
        throw new Error(`金币音效加载失败：HTTP ${response.status}`);
      return response.arrayBuffer();
    })
    .then((bytes) => audioContext.decodeAudioData(bytes));
  await resumed;
  buffer = await loading;
}

export function playCoinArrival() {
  const sound = context.createBufferSource();
  const volume = context.createGain();
  sound.buffer = buffer;
  volume.gain.value = 0.7;
  sound.connect(volume).connect(context.destination);
  sound.onended = () => {
    sound.disconnect();
    volume.disconnect();
  };
  sound.start();
}

export function closeCoinSound() {
  if (context) context.close();
  context = undefined;
  buffer = undefined;
  loading = undefined;
}
