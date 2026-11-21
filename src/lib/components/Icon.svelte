<script lang="ts" module>
  // 16×16 line icons drawn with currentColor.
  const paths = {
    "chevron-right": "M6 4l4 4-4 4",
    "chevron-down": "M4 6l4 4 4-4",
    "chevron-up": "M4 10l4-4 4 4",
    "arrow-up": "M8 13V3M4 7l4-4 4 4",
    "arrow-left": "M13 8H3M7 4L3 8l4 4",
    folder: "M2 4.5A1.5 1.5 0 0 1 3.5 3h3l1.5 1.5h4.5A1.5 1.5 0 0 1 14 6v5.5a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 2 11.5z",
    shelf: "M2 13.5h12M3.5 13.5V4M6 13.5V5.5M8.5 13.5V3.5M10.5 13.5l2-8.5 1.5.4-2 8.1",
    plus: "M8 3v10M3 8h10",
    settings:
      "M8 10a2 2 0 1 0 0-4 2 2 0 0 0 0 4zM13 8a5 5 0 0 0-.1-1l1.3-1-1.2-2-1.5.6a5 5 0 0 0-1.7-1L9.5 2h-3l-.3 1.6a5 5 0 0 0-1.7 1L3 4l-1.2 2 1.3 1a5 5 0 0 0 0 2l-1.3 1L3 12l1.5-.6a5 5 0 0 0 1.7 1l.3 1.6h3l.3-1.6a5 5 0 0 0 1.7-1L13 12l1.2-2-1.3-1a5 5 0 0 0 .1-1z",
    search: "M7 12A5 5 0 1 0 7 2a5 5 0 0 0 0 10zM14 14l-3.5-3.5",
    x: "M4 4l8 8M12 4l-8 8",
    "sort-asc": "M5 13V3M2 6l3-3 3 3M10 4h4M10 8h3M10 12h2",
    "sort-desc": "M5 3v10M2 10l3 3 3-3M10 4h2M10 8h3M10 12h4",
    "grid-s": "M2 2h3v3H2zM6.5 2h3v3h-3zM11 2h3v3h-3zM2 6.5h3v3H2zM6.5 6.5h3v3h-3zM11 6.5h3v3h-3zM2 11h3v3H2zM6.5 11h3v3h-3zM11 11h3v3h-3z",
    "grid-m": "M2.5 2.5h4.5v4.5H2.5zM9 2.5h4.5v4.5H9zM2.5 9h4.5v4.5H2.5zM9 9h4.5v4.5H9z",
    "grid-l": "M2.5 2.5h11v11h-11zM2.5 8h11M8 2.5v11",
    details: "M2 3.5h3v3H2zM7 4h7M7 6h5M2 9.5h3v3H2zM7 10h7M7 12h5",
    compact: "M2 3.5h12M2 6.5h12M2 9.5h12M2 12.5h12",
    subfolders: "M2 3h4l1 1h4v3M4 7h4l1 1h5v5H4z",
    group: "M2 3h12M2 7.5h5M2 10.5h5M2 13.5h5M9 7.5h5v6H9z",
    book: "M3 2.5h7.5a2 2 0 0 1 2 2v9H5a2 2 0 0 1-2-2zM3 11.5a2 2 0 0 1 2-2h7.5",
  } as const;

  export type IconName = keyof typeof paths;
</script>

<script lang="ts">
  let { name, size = 16 }: { name: IconName; size?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 16 16"
  fill="none"
  stroke="currentColor"
  stroke-width="1.4"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={paths[name]} />
</svg>
