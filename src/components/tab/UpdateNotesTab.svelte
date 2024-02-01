<script lang="ts">
  import { type MsaAuthentication, launcherConfigStore } from "$/ipc/stores/launcher_config";

  // Checked at <App />
  let auth = $launcherConfigStore.authentication!;

  let is_premium = (auth as MsaAuthentication).msa_access_token !== undefined;
  let { username, uuid } = auth;
</script>

<main>
  <div class="left">
    <h2>Gelcorp News</h2>
    <br />
    <article>
      <!-- svelte-ignore a11y-invalid-attribute -->
      <p><b><a href="#">Actualización 1.0.0</a></b></p>
      + Se agregó la FLDSMDFR
      <br />
      + Se agregó el Jenny's Mod
      <br />
      - Removed Naza
      <br />
      <img src="https://images7.memedroid.com/images/UPLOADED542/659b0fcfd394d.webp" alt="" />
    </article>
  </div>
  <aside>
    <section>
      <h3>Información del Jugador</h3>
      <div class="user-info">
        <img class="avatar" src="https://crafatar.com/avatars/{uuid}?overlay=true?size=64" alt="avatar" draggable="false" />
        <div class="data">
          <p>{username}</p>
          <p>Cuenta <b>{is_premium ? "Premium" : "No Premium"}</b></p>
        </div>
        <div class="buttons">
          <button on:click={launcherConfigStore.logout}>Cerrar Sesión</button>
        </div>
      </div>
    </section>
  </aside>
</main>

<style>
  * {
    margin: 0;
    padding: 0;
    font-family: sans-serif;
  }

  main {
    font-family: sans-serif;
    color: #e0d0d0;

    background-image: url("/polished_blackstone_bricks.png");
    background-size: 64px;
    image-rendering: pixelated;

    color: white;
    margin: 0;
    padding: 1.5em;
    box-sizing: border-box;
    overflow: auto;

    height: 100%;
    width: 100%;
    display: grid;
    grid-template-columns: 1fr 240px;
    gap: 16px;
  }

  h3 {
    color: #ffffff;
    font-size: 16px;
  }

  .left {
    width: 100%;
  }

  .left a {
    color: #aaaaff;
  }

  .left img {
    max-height: 300px;
  }

  aside {
    text-align: center;
  }

  .avatar {
    width: 3.45em;
    aspect-ratio: 1;
    user-select: none;
  }

  .user-info {
    display: grid;
    justify-content: right;
    gap: 10px;
    grid-template-areas:
      "avatar data"
      "buttons buttons";
    grid-template-rows: 1fr min-content;
    grid-template-columns: min-content 1fr;
    align-content: center;
  }

  .user-info .buttons {
    grid-column: -3 / -1;
    display: flex;
    flex-direction: column;
  }

  .user-info .data {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: start;
  }

  .user-info .data p {
    margin: 0;
    padding: 0;
  }
</style>
