"""Make the shared, original coin-shower effect using damped metallic impacts."""
import math
import random
import shutil
import struct
import wave
from pathlib import Path

root = Path(__file__).resolve().parents[1]
rng = random.Random(20261111)
rate = 44100
duration = 1.85
frames = int(rate * duration)
left, right = [0.0] * frames, [0.0] * frames
hits = [0, .045, .10, .15, .21, .26, .32, .38, .44, .51, .58, .66, .75, .85, .97, 1.10, 1.25]
for hit, start in enumerate(hits):
    frequency = rng.uniform(2050, 3300)
    decay = rng.uniform(.055, .095)
    amplitude = rng.uniform(.19, .28) * (1 - .48 * start / hits[-1])
    pan = rng.uniform(.18, .82)
    phases = [rng.uniform(0, math.tau) for _ in range(5)]
    partials = [(1, 1), (1.483, .54), (2.091, .29), (2.767, .17), (3.91, .085)]
    for n in range(int(rate * .45)):
        index = int(start * rate) + n
        if index >= frames:
            break
        t = n / rate
        attack = min(1, t / .0015)
        tone = sum(
            strength * math.sin(math.tau * frequency * ratio * t + phases[j])
            * math.exp(-t / (decay / (1 + j * .2)))
            for j, (ratio, strength) in enumerate(partials)
        )
        strike = rng.uniform(-1, 1) * .23 * math.exp(-t / .0035)
        sample = amplitude * attack * (tone + strike)
        left[index] += sample * math.sqrt(1 - pan)
        right[index] += sample * math.sqrt(pan)

# Quiet reflections give the short impacts a little space.
for channel in (left, right):
    dry = channel[:]
    for delay, gain in [(.031, .12), (.073, .075), (.127, .04)]:
        offset = int(delay * rate)
        for i in range(offset, frames):
            channel[i] += dry[i - offset] * gain
peak = max(max(abs(v) for v in left), max(abs(v) for v in right))
gain = .76 / peak
samples = bytearray()
for i in range(frames):
    fade = min(1, (frames - i) / (rate * .04))
    samples.extend(struct.pack('<hh', int(left[i] * gain * fade * 32767), int(right[i] * gain * fade * 32767)))
out = root / 'assets/coin-arrival.wav'
with wave.open(str(out), 'wb') as audio:
    audio.setnchannels(2)
    audio.setsampwidth(2)
    audio.setframerate(rate)
    audio.writeframes(samples)
for target in [root / 'web/public/assets/coin-arrival.wav', root / 'android/app/src/main/res/raw/coin_arrival.wav']:
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(out, target)
print(f'{out}: {duration}s, stereo PCM16, {rate}Hz, peak=0.76')
