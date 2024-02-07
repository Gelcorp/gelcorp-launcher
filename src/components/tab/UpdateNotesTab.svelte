<main>
  <iframe title="Notas de actualización" role="presentation" src={import.meta.env.VITE_BLOG_URL} frameborder="0" />
  <h1>Cargando...</h1>
</main>

<style>
  main {
    position: relative;

    width: 100%;
    height: 100%;

    display: grid;
    place-content: center;

    background-color: #404040;
    color: white;
  }

  iframe {
    width: 100%;
    height: 100%;

    position: absolute;
    inset: 0;
  }
</style>
