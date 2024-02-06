<main>
  <div class="container">
    <slot />
  </div>
</main>

<style>
  main {
    background-image: url("/bedrock.png");
    background-color: #0000008a;
    background-blend-mode: multiply;
    background-size: 64px;
    image-rendering: pixelated;
    box-shadow: inset 0 -10px 80px -5px black;

    display: grid;
    place-content: center;
  }

  .container {
    color: white;
    background-color: #0f0f0f;
    padding: 1em;

    display: flex;
    flex-direction: column;
    align-items: stretch;
    width: 42vw;
    max-width: 340px;
    text-align: center;
  }
</style>
