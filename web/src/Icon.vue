<script setup>
defineProps({ name: String, size: { default: 22 } });
const paths = {
  play: "m8 4 12 8-12 8V4Z",
  pause: "M7 4h3v16H7z M14 4h3v16h-3z",
  relation:
    "M12 3a3 3 0 1 1 0 6 3 3 0 0 1 0-6Z M5 15a3 3 0 1 1 0 6 3 3 0 0 1 0-6Z M19 15a3 3 0 1 1 0 6 3 3 0 0 1 0-6Z M12 9v3 M5 15v-3h14v3",
  hall: "M4 10V5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v11a1 1 0 0 1-1 1h-9l-6 4V10 M8 8h8 M8 12h5",
  bag: "M5 7h14l2 14H3L5 7Z M8 8V6a4 4 0 0 1 8 0v2",
  user: "M16 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0Z M4 21v-2a8 8 0 0 1 16 0v2",
  arrow: "m9 5 7 7-7 7",
  back: "m15 5-7 7 7 7",
  close: "m6 6 12 12 M18 6 6 18",
  eye: "M2 12s3-7 10-7 10 7 10 7-3 7-10 7S2 12 2 12Z M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z",
  "eye-off":
    "m3 3 18 18 M10 5c8-1 12 7 12 7s-1 3-4 5 M6 6c-3 2-4 6-4 6s3 7 10 7c2 0 3-1 4-1 M10 10a3 3 0 0 0 4 4",
  gift: "M3 10h18v11H3z M2 6h20v4H2z M12 6v15 M12 6H8a3 3 0 1 1 3-3l1 3Z M12 6h4a3 3 0 1 0-3-3l-1 3Z",
  coin: "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z M9 8l3 4 3-4 M9 12h6 M9 15h6 M12 12v6",
  calendar: "M4 5h16v16H4z M8 2v6 M16 2v6 M4 10h16 m-12 5 3 3 5-5",
  wallet: "M3 5h16v4H3z M3 5v15h18V9H3 M16 13h5v4h-5z",
  list: "M8 5h13 M8 12h13 M8 19h13 M3 5h.1 M3 12h.1 M3 19h.1",
  settings:
    "M12 8a4 4 0 1 1 0 8 4 4 0 0 1 0-8Z M9 3h6l1 3 3 1 2 5-2 5-3 1-1 3H9l-1-3-3-1-2-5 2-5 3-1 1-3Z",
  logout: "M9 3H3v18h6 M8 12h13 m-5-5 5 5-5 5",
  bell: "M5 17h14l-2-4V8A5 5 0 0 0 7 8v5l-2 4Z M10 21h4",
  spark: "m12 2 2.8 7.2L22 12l-7.2 2.8L12 22l-2.8-7.2L2 12l7.2-2.8L12 2Z",
  check: "m5 12 4 4 10-10",
  grid: "M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z",
  send: "m3 3 19 9-19 9 4-9-4-9Z M7 12h15",
  refresh: "M20 8a8 8 0 1 0 0 8 M20 3v5h-5",
  download: "M12 3v12 m-5-5 5 5 5-5 M4 16v5h16v-5",
  mail: "M3 5h18v14H3Z M3 5l9 7 9-7",
  shield: "m12 2 8 4v6c0 5-8 10-8 10S4 17 4 12V6l8-4Z m-4 10 3 3 5-6",
  search: "M16 10a6 6 0 1 1-12 0 6 6 0 0 1 12 0Z m-2 4 7 7",
  chevron: "m6 9 6 6 6-6",
};
</script>
<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.7"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path :d="paths[name] || paths.spark" />
  </svg>
</template>
