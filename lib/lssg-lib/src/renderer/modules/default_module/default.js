function toggleModal(e) {
  const modal = document.getElementById("default__modal");
  if (modal !== null) {
    modal.remove();
    return;
  }
  const html = document.createElement("div");
  html.id = "default__modal";
  html.innerHTML = `<div id="default__modal">
      <span id="default__modal_close">&times;</span>
      <img id="default__modal_image" />
      <div id="default__modal_caption">${e.target.alt}</div>
    </div>`;
  document.body.appendChild(html);
  document.getElementById("default__modal_image").src = e.target.src;
  document
    .getElementById("default__modal")
    .addEventListener("click", toggleModal);
}

for (const e of document.getElementsByClassName("default__carausel_item")) {
  e.addEventListener("click", toggleModal);
}
