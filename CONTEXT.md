# Continuum

A GTK4 desktop reader for manhwa/manga stored as `.cbz` archives, with continuous scrolling across an entire series.

## Language

**Chapter**:
One `.cbz` archive — a single comic installment. Cross-chapter navigation is seamless.
_Avoid_: file, archive, comic

**Series**:
The set of sibling `.cbz` files in the directory containing the opened chapter. The reader auto-loads chapters from the series as you scroll.
_Avoid_: directory, collection

**Page**:
One image entry inside a chapter archive. Pages are displayed at full resolution, lazy-loaded around the viewport.
_Avoid_: image, panel, slice

**Reader**:
The scrollable view that presents the series. Two reading modes exist:
_Avoid_: session, document

**Webtoon mode**:
Continuous vertical scrolling; each page fit to the width, height following the page's native aspect ratio.
_Avoid_: vertical mode

**Volume view**:
Horizontal scrolling, LTR; each page fit to the viewport height so a full page needs no vertical scrolling.
_Avoid_: manga mode, horizontal mode, paged mode

**Progress**:
The reader's record of which page of which chapter the user last reached, emitted to an external consumer on exit.
