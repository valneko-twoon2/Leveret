<script lang="ts">
  import logo from '../assets/leveret.svg'

  let { size = 24 }: { size?: number } = $props()
</script>

<img src={logo} alt="" width={size} height={size} />

<style>
  img {
    flex: none;
    display: block;
  }
</style>
