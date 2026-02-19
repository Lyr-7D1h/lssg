const default__imageGalleryState = {
  images: [],
  index: -1,
  bodyOverflow: "",
  zoom: 1,
  zoomOriginX: 50,
  zoomOriginY: 50,
  lastSwipeAt: 0,
};

function default__applyImageModalZoom() {
  const modal = document.getElementById("default__modal");
  if (!modal) return;

  const image = modal.querySelector("#default__modal_image");
  if (!image) return;

  image.style.transformOrigin = `${default__imageGalleryState.zoomOriginX}% ${default__imageGalleryState.zoomOriginY}%`;
  image.style.transform = `scale(${default__imageGalleryState.zoom})`;
  modal.classList.toggle(
    "default__modal_zoomed",
    default__imageGalleryState.zoom > 1,
  );
}

function default__setImageModalZoom(nextZoom) {
  const clamped = Math.max(1, Math.min(8, nextZoom));
  default__imageGalleryState.zoom = clamped;
  default__applyImageModalZoom();
}

function default__resetImageModalZoom() {
  default__imageGalleryState.zoomOriginX = 50;
  default__imageGalleryState.zoomOriginY = 50;
  default__setImageModalZoom(1);
}

function default__setImageModalZoomOriginFromClick(e, image) {
  const rect = image.getBoundingClientRect();
  if (!rect.width || !rect.height) return;

  const x = ((e.clientX - rect.left) / rect.width) * 100;
  const y = ((e.clientY - rect.top) / rect.height) * 100;

  default__imageGalleryState.zoomOriginX = Math.max(0, Math.min(100, x));
  default__imageGalleryState.zoomOriginY = Math.max(0, Math.min(100, y));
}

function default__lockBodyScroll() {
  default__imageGalleryState.bodyOverflow = document.body.style.overflow;
  document.body.style.overflow = "hidden";
}

function default__unlockBodyScroll() {
  document.body.style.overflow = default__imageGalleryState.bodyOverflow || "";
}

function default__collectGalleryImages() {
  return Array.from(
    document.querySelectorAll("#default__content .default__carousel img"),
  ).filter((img) => !img.closest(".default__carousel_thumbs_viewport"));
}

function default__isModalOpen() {
  const modal = document.getElementById("default__modal");
  return !!modal && modal.style.display !== "none";
}

function default__closeImageModal() {
  const modal = document.getElementById("default__modal");
  if (!modal) return;

  const image = modal.querySelector("#default__modal_image");
  const caption = modal.querySelector("#default__modal_caption");
  const counter = modal.querySelector("#default__modal_counter");

  modal.style.display = "none";
  default__unlockBodyScroll();
  default__resetImageModalZoom();
  if (image) image.removeAttribute("src");
  if (caption) caption.textContent = "";
  if (counter) counter.textContent = "";

  default__imageGalleryState.images = [];
  default__imageGalleryState.index = -1;
}

function default__updateImageModalContent() {
  const modal = document.getElementById("default__modal");
  if (!modal) return;

  const image = modal.querySelector("#default__modal_image");
  const caption = modal.querySelector("#default__modal_caption");
  const counter = modal.querySelector("#default__modal_counter");
  const prev = modal.querySelector("#default__modal_prev");
  const next = modal.querySelector("#default__modal_next");
  const { images, index } = default__imageGalleryState;

  if (!image || !caption || !counter || !images.length || index < 0) return;

  const current = images[index];
  const src = current.currentSrc || current.src;
  const captionText =
    current.getAttribute("alt") || current.getAttribute("title") || "";

  image.src = src;
  caption.textContent = captionText;
  counter.textContent = `${index + 1} / ${images.length}`;
  default__resetImageModalZoom();

  if (prev) prev.disabled = index === 0;
  if (next) next.disabled = index === images.length - 1;
}

function default__imageModalPrev() {
  if (default__imageGalleryState.index <= 0) return;
  default__imageGalleryState.index -= 1;
  default__updateImageModalContent();
}

function default__imageModalNext() {
  if (
    default__imageGalleryState.index >=
    default__imageGalleryState.images.length - 1
  ) {
    return;
  }
  default__imageGalleryState.index += 1;
  default__updateImageModalContent();
}

function default__getOrCreateImageModal() {
  let modal = document.getElementById("default__modal");
  if (modal) return modal;

  modal = document.createElement("div");
  modal.id = "default__modal";
  modal.style.display = "none";

  const close = document.createElement("span");
  close.id = "default__modal_close";
  close.textContent = "×";

  const image = document.createElement("img");
  image.id = "default__modal_image";
  image.setAttribute("draggable", "false");

  const prev = document.createElement("button");
  prev.id = "default__modal_prev";
  prev.className = "default__modal_nav";
  prev.setAttribute("aria-label", "Previous image");

  const next = document.createElement("button");
  next.id = "default__modal_next";
  next.className = "default__modal_nav";
  next.setAttribute("aria-label", "Next image");

  const caption = document.createElement("div");
  caption.id = "default__modal_caption";

  const counter = document.createElement("div");
  counter.id = "default__modal_counter";

  modal.appendChild(close);
  modal.appendChild(prev);
  modal.appendChild(next);
  modal.appendChild(image);
  modal.appendChild(caption);
  modal.appendChild(counter);
  document.body.appendChild(modal);

  close.addEventListener("click", default__closeImageModal);
  prev.addEventListener("click", (e) => {
    e.stopPropagation();
    default__imageModalPrev();
  });
  next.addEventListener("click", (e) => {
    e.stopPropagation();
    default__imageModalNext();
  });
  modal.addEventListener("click", (e) => {
    if (e.target === modal) default__closeImageModal();
  });
  image.addEventListener(
    "wheel",
    (e) => {
      if (!default__isModalOpen()) return;
      e.preventDefault();
      e.stopPropagation();

      const step = e.deltaY < 0 ? 0.25 : -0.25;
      default__setImageModalZoom(default__imageGalleryState.zoom + step);
    },
    { passive: false },
  );
  image.addEventListener("dblclick", (e) => {
    if (!default__isModalOpen()) return;
    e.preventDefault();
    e.stopPropagation();
    default__setImageModalZoom(default__imageGalleryState.zoom > 1 ? 1 : 2);
  });
  image.addEventListener("click", (e) => {
    if (!default__isModalOpen()) return;

    if (Date.now() - default__imageGalleryState.lastSwipeAt < 350) {
      return;
    }

    e.preventDefault();
    e.stopPropagation();

    if (default__imageGalleryState.zoom > 1) {
      default__resetImageModalZoom();
      return;
    }

    default__setImageModalZoomOriginFromClick(e, image);
    default__setImageModalZoom(3);
  });
  modal.addEventListener("wheel", (e) => e.preventDefault(), {
    passive: false,
  });

  let touchStartX = 0;
  let touchStartY = 0;

  modal.addEventListener(
    "touchstart",
    (e) => {
      if (!default__isModalOpen() || e.touches.length !== 1) return;
      touchStartX = e.touches[0].clientX;
      touchStartY = e.touches[0].clientY;
    },
    { passive: true },
  );

  modal.addEventListener(
    "touchend",
    (e) => {
      if (!default__isModalOpen() || e.changedTouches.length !== 1) return;
      if (default__imageGalleryState.zoom > 1) return;

      const endX = e.changedTouches[0].clientX;
      const endY = e.changedTouches[0].clientY;
      const diffX = endX - touchStartX;
      const diffY = endY - touchStartY;

      if (Math.abs(diffX) < 50 || Math.abs(diffX) <= Math.abs(diffY)) return;

      default__imageGalleryState.lastSwipeAt = Date.now();
      if (diffX > 0) {
        default__imageModalPrev();
      } else {
        default__imageModalNext();
      }
    },
    { passive: true },
  );

  modal.addEventListener("touchmove", (e) => e.preventDefault(), {
    passive: false,
  });
  document.addEventListener("keydown", (e) => {
    if (!default__isModalOpen()) return;
    if (e.key === "Escape") {
      default__closeImageModal();
      return;
    }
    if (e.key === "ArrowLeft") {
      default__imageModalPrev();
      return;
    }
    if (e.key === "ArrowRight") {
      default__imageModalNext();
      return;
    }
    if (e.key === "+" || e.key === "=") {
      e.preventDefault();
      default__setImageModalZoom(default__imageGalleryState.zoom + 0.25);
      return;
    }
    if (e.key === "-") {
      e.preventDefault();
      default__setImageModalZoom(default__imageGalleryState.zoom - 0.25);
      return;
    }
    if (e.key === "0") {
      e.preventDefault();
      default__resetImageModalZoom();
    }
  });

  return modal;
}

function default__openImageModal(clickedImage) {
  if (!clickedImage) return;

  const modal = default__getOrCreateImageModal();
  const images = default__collectGalleryImages();
  const index = images.findIndex((img) => img === clickedImage);
  if (index < 0) return;

  default__imageGalleryState.images = images;
  default__imageGalleryState.index = index;
  default__updateImageModalContent();
  default__lockBodyScroll();
  modal.style.display = "flex";
}

function default__carouselCurrentIndex(carousel) {
  const container = carousel.querySelector(".default__carousel_container");
  if (!container) return 0;

  const currentTransform = container.style.transform || "translate3d(0%, 0, 0)";
  const match = currentTransform.match(/translate3d\((-?\d+)%/);
  const currentOffset = match ? parseInt(match[1]) : 0;
  return Math.abs(currentOffset / 100);
}

function default__carouselGoTo(e, index, skipScroll = false) {
  const carousel = e.target.closest(".default__carousel");
  if (!carousel) return;

  const container = carousel.querySelector(".default__carousel_container");
  const slides = carousel.querySelectorAll(".default__carousel_slide");
  const thumbs = carousel.querySelectorAll(".default__carousel_thumb");

  if (!container || !slides.length) return;

  // Update transform to show the selected slide
  const offset = -index * 100;
  container.style.transform = `translate3d(${offset}%, 0, 0)`;

  // Update thumbnails
  thumbs.forEach((thumb, i) => {
    if (i === index) {
      thumb.classList.add("is-active");
      if (!skipScroll) {
        thumb.scrollIntoView({
          behavior: "smooth",
          block: "nearest",
          inline: "center",
        });
      }
    } else {
      thumb.classList.remove("is-active");
    }
  });

  // Update navigation buttons
  const prevBtn = carousel.querySelector(".default__carousel_btn_prev");
  const nextBtn = carousel.querySelector(".default__carousel_btn_next");
  const zoomBtn = carousel.querySelector(".default__carousel_btn_zoom");
  if (prevBtn) prevBtn.disabled = index === 0;
  if (nextBtn) nextBtn.disabled = index === slides.length - 1;
  if (zoomBtn) {
    const currentSlide = slides[index];
    const hasImage = !!currentSlide.querySelector("img");
    const canZoom = hasImage;
    zoomBtn.disabled = !canZoom;
    carousel.classList.toggle("default__carousel_can_zoom", canZoom);
  } else {
    carousel.classList.remove("default__carousel_can_zoom");
  }
}

function default__carouselPrev(e) {
  const carousel = e.target.closest(".default__carousel");
  if (!carousel) return;

  const currentIndex = default__carouselCurrentIndex(carousel);

  if (currentIndex > 0) {
    const thumb = carousel.querySelector(
      `.default__carousel_thumb[data-index="${currentIndex - 1}"]`,
    );
    if (thumb) {
      default__carouselGoTo({ target: thumb }, currentIndex - 1);
    }
  }
}

function default__carouselNext(e) {
  const carousel = e.target.closest(".default__carousel");
  if (!carousel) return;

  const slides = carousel.querySelectorAll(".default__carousel_slide");
  if (!slides.length) return;

  const currentIndex = default__carouselCurrentIndex(carousel);

  if (currentIndex < slides.length - 1) {
    const thumb = carousel.querySelector(
      `.default__carousel_thumb[data-index="${currentIndex + 1}"]`,
    );
    if (thumb) {
      default__carouselGoTo({ target: thumb }, currentIndex + 1);
    }
  }
}

function default__carouselZoom(e) {
  const carousel = e.target.closest(".default__carousel");
  if (!carousel) return;

  const slides = carousel.querySelectorAll(".default__carousel_slide");
  if (!slides.length) return;

  const currentIndex = default__carouselCurrentIndex(carousel);
  const currentSlide = slides[currentIndex];
  if (!currentSlide) return;

  const image = currentSlide.querySelector("img");
  if (!image) return;

  e.preventDefault();
  e.stopPropagation();
  default__openImageModal(image);
}

// Initialize carousel on load
document.addEventListener("DOMContentLoaded", function () {
  document.addEventListener("click", (e) => {
    const image = e.target.closest("#default__content img");
    if (!image) return;

    if (!image.closest(".default__carousel")) {
      return;
    }

    if (e.target.closest(".default__carousel_btn, .default__carousel_thumb")) {
      return;
    }

    e.preventDefault();
    e.stopPropagation();

    default__openImageModal(image);
  });

  document.querySelectorAll(".default__carousel").forEach((carousel) => {
    const firstThumb = carousel.querySelector(
      '.default__carousel_thumb[data-index="0"]',
    );
    if (firstThumb) {
      default__carouselGoTo({ target: firstThumb }, 0, true);
    }

    // Add drag/swipe functionality
    const viewport = carousel.querySelector(".default__carousel_viewport");
    const container = carousel.querySelector(".default__carousel_container");
    const slides = carousel.querySelectorAll(".default__carousel_slide");

    if (!viewport || !container || !slides.length) return;

    let isDragging = false;
    let startX = 0;
    let currentX = 0;
    let startTransform = 0;
    let currentIndex = 0;

    const getCurrentIndex = () => {
      const transform = container.style.transform || "translate3d(0%, 0, 0)";
      const match = transform.match(/translate3d\((-?\d+)%/);
      return match ? Math.abs(parseInt(match[1]) / 100) : 0;
    };

    // Mobile: Touch swipe functionality
    const handleStart = (e) => {
      isDragging = true;
      startX = e.touches[0].pageX;
      currentX = startX;
      currentIndex = getCurrentIndex();
      startTransform = -currentIndex * 100;
      container.style.transition = "none";
    };

    const handleMove = (e) => {
      if (!isDragging) return;

      currentX = e.touches[0].pageX;
      const diffX = currentX - startX;

      // Only prevent default and drag if moved more than 3px
      if (Math.abs(diffX) < 3) return;

      e.preventDefault();

      const diffPercent = (diffX / viewport.offsetWidth) * 100;
      const newTransform = startTransform + diffPercent;

      // Add resistance at boundaries
      const minTransform = -(slides.length - 1) * 100;
      const maxTransform = 0;

      if (newTransform > maxTransform) {
        container.style.transform = `translate3d(${maxTransform + (newTransform - maxTransform) * 0.3}%, 0, 0)`;
      } else if (newTransform < minTransform) {
        container.style.transform = `translate3d(${minTransform + (newTransform - minTransform) * 0.3}%, 0, 0)`;
      } else {
        container.style.transform = `translate3d(${newTransform}%, 0, 0)`;
      }
    };

    const handleEnd = (e) => {
      if (!isDragging) return;
      isDragging = false;

      const diffX = currentX - startX;
      const threshold = viewport.offsetWidth * 0.1;

      container.style.transition =
        "transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94)";

      let newIndex = currentIndex;

      if (diffX > threshold && currentIndex > 0) {
        // Swiped right, go to previous
        newIndex = currentIndex - 1;
      } else if (diffX < -threshold && currentIndex < slides.length - 1) {
        // Swiped left, go to next
        newIndex = currentIndex + 1;
      }

      const thumb = carousel.querySelector(
        `.default__carousel_thumb[data-index="${newIndex}"]`,
      );
      if (thumb) {
        default__carouselGoTo({ target: thumb }, newIndex);
      } else {
        // Snap back to current
        container.style.transform = `translate3d(${-currentIndex * 100}%, 0, 0)`;
      }
    };

    // Touch events for mobile
    viewport.addEventListener("touchstart", handleStart, { passive: true });
    viewport.addEventListener("touchmove", handleMove, { passive: false });
    viewport.addEventListener("touchend", handleEnd);
  });
});
