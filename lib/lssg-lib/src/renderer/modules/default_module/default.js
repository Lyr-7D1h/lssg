function default__carauselGoTo(e, index, skipScroll = false) {
  const carousel = e.target.closest(".default__carausel");
  if (!carousel) return;

  const container = carousel.querySelector(".default__carausel_container");
  const slides = carousel.querySelectorAll(".default__carausel_slide");
  const thumbs = carousel.querySelectorAll(".default__carausel_thumb");

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
  const prevBtn = carousel.querySelector(".default__carausel_btn_prev");
  const nextBtn = carousel.querySelector(".default__carausel_btn_next");
  if (prevBtn) prevBtn.disabled = index === 0;
  if (nextBtn) nextBtn.disabled = index === slides.length - 1;
}

function default__carauselPrev(e) {
  const carousel = e.target.closest(".default__carausel");
  if (!carousel) return;

  const container = carousel.querySelector(".default__carausel_container");
  if (!container) return;

  const currentTransform = container.style.transform || "translate3d(0%, 0, 0)";
  const match = currentTransform.match(/translate3d\((-?\d+)%/);
  const currentOffset = match ? parseInt(match[1]) : 0;
  const currentIndex = Math.abs(currentOffset / 100);

  if (currentIndex > 0) {
    const thumb = carousel.querySelector(
      `.default__carausel_thumb[data-index="${currentIndex - 1}"]`,
    );
    if (thumb) {
      default__carauselGoTo({ target: thumb }, currentIndex - 1);
    }
  }
}

function default__carauselNext(e) {
  const carousel = e.target.closest(".default__carausel");
  if (!carousel) return;

  const container = carousel.querySelector(".default__carausel_container");
  const slides = carousel.querySelectorAll(".default__carausel_slide");
  if (!container || !slides.length) return;

  const currentTransform = container.style.transform || "translate3d(0%, 0, 0)";
  const match = currentTransform.match(/translate3d\((-?\d+)%/);
  const currentOffset = match ? parseInt(match[1]) : 0;
  const currentIndex = Math.abs(currentOffset / 100);

  if (currentIndex < slides.length - 1) {
    const thumb = carousel.querySelector(
      `.default__carausel_thumb[data-index="${currentIndex + 1}"]`,
    );
    if (thumb) {
      default__carauselGoTo({ target: thumb }, currentIndex + 1);
    }
  }
}

// Initialize carousel on load
document.addEventListener("DOMContentLoaded", function () {
  document.querySelectorAll(".default__carausel").forEach((carousel) => {
    const firstThumb = carousel.querySelector(
      '.default__carausel_thumb[data-index="0"]',
    );
    if (firstThumb) {
      default__carauselGoTo({ target: firstThumb }, 0, true);
    }

    // Add drag/swipe functionality
    const viewport = carousel.querySelector(".default__carausel_viewport");
    const container = carousel.querySelector(".default__carausel_container");
    const slides = carousel.querySelectorAll(".default__carausel_slide");

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

    // Desktop: Click left/right sides to navigate
    viewport.addEventListener("click", (e) => {
      if (isDragging) return; // Don't trigger on drag

      // Don't navigate if clicking on interactive elements like model-viewer
      if (e.target.closest("model-viewer")) return;

      const rect = viewport.getBoundingClientRect();
      const clickX = e.clientX - rect.left;
      const halfWidth = rect.width / 2;

      currentIndex = getCurrentIndex();

      if (clickX < halfWidth && currentIndex > 0) {
        // Clicked left half - go to previous
        const thumb = carousel.querySelector(
          `.default__carausel_thumb[data-index="${currentIndex - 1}"]`,
        );
        if (thumb) default__carauselGoTo({ target: thumb }, currentIndex - 1);
      } else if (clickX >= halfWidth && currentIndex < slides.length - 1) {
        // Clicked right half - go to next
        const thumb = carousel.querySelector(
          `.default__carausel_thumb[data-index="${currentIndex + 1}"]`,
        );
        if (thumb) default__carauselGoTo({ target: thumb }, currentIndex + 1);
      }
    });

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
        `.default__carausel_thumb[data-index="${newIndex}"]`,
      );
      if (thumb) {
        default__carauselGoTo({ target: thumb }, newIndex);
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
