<?php

require_once 'config.php';

ob_start();

function dumpToLog() {
  $text = ob_get_contents();
  ob_end_clean();
  $logFileName = getenv("TEMP") . '/request.log';
  $log = fopen($logFileName, 'a+');
  fwrite($log, "==================\n");
  fwrite($log, $text);
  fwrite($log, "==================\n");
  fclose($log);
}

register_shutdown_function('dumpToLog');