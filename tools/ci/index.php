<?php

require_once ('common.php');

$inputJSON = file_get_contents('php://input');
$body = json_decode($inputJSON, true);

echo "Request body:\n";
var_dump($body);
